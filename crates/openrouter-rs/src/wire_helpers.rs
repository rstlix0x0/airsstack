//! Shared wire-layer helpers used when processing an HTTP response.
//!
//! Groups the body-collection and error-decoding routines a resource needs
//! after the transport returns. Kept in one place so each resource module
//! stays focused on request construction and status interpretation, and so a
//! second resource reuses the same decoding without duplication.
//!
//! Responsibilities:
//! - [`collect_body`] — drain a [`crate::transport::BodyStream`] into bytes,
//!   enforcing a size cap.
//! - [`decode_api_error_from_parts`] — turn a non-2xx status + headers + body
//!   into an [`crate::error::Error`], routing the rate-limit, moderation,
//!   provider-passthrough, generic-API, and undecodable cases.
//!
//! Not responsible for:
//! - Constructing HTTP requests or setting headers — the resource layer does.
//! - Serializing request bodies — also the resource layer.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::time::Duration;

use crate::error::Error;
use crate::headers as h;
pub(crate) use airs_transport::{MAX_RESPONSE_BODY_BYTES, collect_body};

/// Outer error envelope every non-2xx body uses: `{"error":{...}}`.
#[derive(serde::Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

/// Inner error object: a numeric `code`, a `message`, and optional `metadata`.
#[derive(serde::Deserialize)]
struct ApiErrorBody {
    code: i32,
    message: String,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

/// Moderation-block metadata shape (HTTP 403). The presence of `reasons`
/// distinguishes it from a provider-passthrough error.
#[derive(serde::Deserialize)]
struct ModerationMeta {
    reasons: Vec<String>,
    flagged_input: String,
    provider_name: String,
    model_slug: String,
}

/// Provider-passthrough metadata shape. `raw` is the upstream provider's
/// untyped error payload; its presence distinguishes it from moderation.
#[derive(serde::Deserialize)]
struct ProviderMeta {
    provider_name: String,
    raw: serde_json::Value,
}

/// Decode a non-2xx HTTP response into an [`Error`].
///
/// Extracts the `Retry-After` header, then routes the decoded error envelope
/// by status and metadata shape into the rate-limit, moderation,
/// provider-passthrough, or generic-API case. A body that is not a recognized
/// error envelope becomes [`Error::UndecodableApiError`].
pub(crate) fn decode_api_error_from_parts(
    status: http::StatusCode,
    headers: &http::HeaderMap,
    body_bytes: &[u8],
) -> Error {
    let retry_after = headers
        .get(h::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_retry_after);

    let body = match serde_json::from_slice::<ApiErrorEnvelope>(body_bytes) {
        Ok(envelope) => envelope.error,
        Err(_) => {
            return Error::UndecodableApiError {
                status: status.as_u16(),
                detail: String::from_utf8_lossy(body_bytes).into_owned(),
            };
        }
    };

    let code = status.as_u16();

    if code == 429 {
        return Error::RateLimit { retry_after };
    }

    if code == 403 {
        if let Some(m) = body
            .metadata
            .as_ref()
            .and_then(|md| serde_json::from_value::<ModerationMeta>(md.clone()).ok())
        {
            return Error::Moderation {
                reasons: m.reasons,
                flagged_input: m.flagged_input,
                provider_name: m.provider_name,
                model_slug: m.model_slug,
            };
        }
    }

    if let Some(p) = body
        .metadata
        .as_ref()
        .and_then(|md| serde_json::from_value::<ProviderMeta>(md.clone()).ok())
    {
        return Error::Provider {
            provider_name: p.provider_name,
            raw: p.raw,
        };
    }

    Error::Api {
        status: code,
        code: body.code,
        message: body.message,
        metadata: body.metadata,
    }
}

/// Parse a `Retry-After` header value as an integer number of seconds.
///
/// Returns `None` when the value is not a non-negative integer (e.g. the
/// HTTP-date form, which the API does not use).
fn parse_retry_after(value: &str) -> Option<Duration> {
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::panic,
        reason = "test-only panics on wrong-variant matches; a panic is the intended failure signal"
    )]

    use super::*;
    use crate::error::Error;
    use http::StatusCode;

    fn headers_with(name: &'static str, value: &str) -> http::HeaderMap {
        let mut m = http::HeaderMap::new();
        m.insert(name, value.parse().unwrap());
        m
    }

    #[test]
    fn generic_non_2xx_decodes_to_api() {
        let body = br#"{"error":{"code":400,"message":"bad request"}}"#;
        let err =
            decode_api_error_from_parts(StatusCode::BAD_REQUEST, &http::HeaderMap::new(), body);
        match err {
            Error::Api {
                status,
                code,
                message,
                metadata,
            } => {
                assert_eq!(status, 400);
                assert_eq!(code, 400);
                assert_eq!(message, "bad request");
                assert!(metadata.is_none());
            }
            other => panic!("expected Api, got {other:?}"),
        }
    }

    #[test]
    fn status_429_decodes_to_rate_limit_with_retry_after() {
        let body = br#"{"error":{"code":429,"message":"slow down"}}"#;
        let headers = headers_with(h::RETRY_AFTER, "12");
        let err = decode_api_error_from_parts(StatusCode::TOO_MANY_REQUESTS, &headers, body);
        match err {
            Error::RateLimit { retry_after } => {
                assert_eq!(retry_after, Some(std::time::Duration::from_secs(12)));
            }
            other => panic!("expected RateLimit, got {other:?}"),
        }
    }

    #[test]
    fn status_403_with_moderation_metadata_decodes_to_moderation() {
        let body = br#"{"error":{"code":403,"message":"flagged","metadata":{
            "reasons":["harassment"],"flagged_input":"...","provider_name":"openai",
            "model_slug":"openai/gpt-4o"}}}"#;
        let err = decode_api_error_from_parts(StatusCode::FORBIDDEN, &http::HeaderMap::new(), body);
        match err {
            Error::Moderation {
                reasons,
                flagged_input,
                provider_name,
                model_slug,
            } => {
                assert_eq!(reasons, vec!["harassment".to_string()]);
                assert_eq!(flagged_input, "...");
                assert_eq!(provider_name, "openai");
                assert_eq!(model_slug, "openai/gpt-4o");
            }
            other => panic!("expected Moderation, got {other:?}"),
        }
    }

    #[test]
    fn generic_403_without_moderation_metadata_decodes_to_api() {
        let body = br#"{"error":{"code":403,"message":"forbidden"}}"#;
        let err = decode_api_error_from_parts(StatusCode::FORBIDDEN, &http::HeaderMap::new(), body);
        assert!(matches!(err, Error::Api { status: 403, .. }));
    }

    #[test]
    fn provider_metadata_decodes_to_provider() {
        let body = br#"{"error":{"code":502,"message":"upstream","metadata":{
            "provider_name":"anthropic","raw":{"type":"overloaded"}}}}"#;
        let err =
            decode_api_error_from_parts(StatusCode::BAD_GATEWAY, &http::HeaderMap::new(), body);
        match err {
            Error::Provider { provider_name, raw } => {
                assert_eq!(provider_name, "anthropic");
                assert_eq!(raw, serde_json::json!({"type":"overloaded"}));
            }
            other => panic!("expected Provider, got {other:?}"),
        }
    }

    #[test]
    fn undecodable_body_decodes_to_undecodable_api_error() {
        let body = b"this is not json";
        let err = decode_api_error_from_parts(
            StatusCode::INTERNAL_SERVER_ERROR,
            &http::HeaderMap::new(),
            body,
        );
        match err {
            Error::UndecodableApiError { status, detail } => {
                assert_eq!(status, 500);
                assert_eq!(detail, "this is not json");
            }
            other => panic!("expected UndecodableApiError, got {other:?}"),
        }
    }

    #[test]
    fn parse_retry_after_parses_integer_and_rejects_junk() {
        assert_eq!(
            parse_retry_after("30"),
            Some(std::time::Duration::from_secs(30))
        );
        assert!(parse_retry_after("not-a-number").is_none());
        assert!(parse_retry_after("").is_none());
    }
}
