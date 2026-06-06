//! Shared API-error-decoding helper used by multiple resource modules.
//!
//! Groups the non-2xx response decoding that every resource module
//! (messages, models) needs when interpreting an HTTP response. Placing it
//! here avoids duplication and keeps each resource focused on request
//! construction and response interpretation. Draining the response body is a
//! transport-layer concern — see [`crate::transport::collect_body`].
//!
//! Responsibilities:
//! - [`decode_api_error_from_parts`] — turn a non-2xx status + headers +
//!   body into an [`crate::error::Error`], extracting `request-id`,
//!   `anthropic-organization-id`, and `retry-after` header values.
//!
//! Not responsible for:
//! - Draining response bodies — that is [`crate::transport::collect_body`].
//! - Constructing HTTP requests or setting headers — the resource layer
//!   handles that.
//! - Serializing request bodies — also the resource layer.
#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

use std::time::Duration;

use crate::error::{ApiError, ApiErrorBody, Error};
use crate::headers as h;
use crate::types::{OrganizationId, RequestId};

/// Outer error envelope the Anthropic API wraps every non-2xx body in.
///
/// Wire format: `{"type":"error","error":{...}}`. The outer `"type":"error"`
/// field is consumed here; the inner object maps to [`ApiErrorBody`].
#[derive(serde::Deserialize)]
struct ApiErrorEnvelope {
    error: ApiErrorBody,
}

/// Decode a non-2xx HTTP response into an [`Error`].
///
/// Extracts `request-id`, `anthropic-organization-id`, and `retry-after`
/// header values from `headers`, then attempts to parse `body_bytes` as an
/// Anthropic API error envelope. Falls back to
/// [`Error::UndecodableApiError`] when the body is not a recognized
/// envelope.
pub(crate) fn decode_api_error_from_parts(
    status: http::StatusCode,
    headers: &http::HeaderMap,
    body_bytes: &[u8],
) -> Error {
    let request_id = headers
        .get(h::REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| RequestId::new(s).ok());

    let organization_id = headers
        .get(h::ANTHROPIC_ORG_ID)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| OrganizationId::new(s).ok());

    let retry_after = headers
        .get(h::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_retry_after);

    decode_error_body(status, body_bytes, request_id, organization_id, retry_after)
}

/// Decode a non-2xx response body into an [`Error`].
///
/// Attempts to parse the body as an [`ApiErrorEnvelope`]; falls back to
/// [`Error::UndecodableApiError`] when the body is not a recognized
/// envelope.
fn decode_error_body(
    status: http::StatusCode,
    body_bytes: &[u8],
    request_id: Option<RequestId>,
    organization_id: Option<OrganizationId>,
    retry_after: Option<Duration>,
) -> Error {
    match serde_json::from_slice::<ApiErrorEnvelope>(body_bytes) {
        Ok(envelope) => Error::Api(ApiError {
            status,
            body: envelope.error,
            request_id,
            organization_id,
            retry_after,
        }),
        Err(_) => Error::UndecodableApiError {
            status,
            detail: String::from_utf8_lossy(body_bytes).into_owned(),
            request_id,
        },
    }
}

/// Parse a `Retry-After` header value as an integer number of seconds.
///
/// Returns `None` when the value is not a non-negative integer (e.g. an
/// HTTP-date format, which is not returned by the current Anthropic API).
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

    #[test]
    fn decode_error_body_valid_envelope_produces_api_error() {
        use crate::error::{ApiError, ErrorType};
        use crate::types::{OrganizationId, RequestId};
        use http::StatusCode;

        let body = br#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;
        let request_id = Some(RequestId::new("req_abc").unwrap());
        let organization_id = Some(OrganizationId::new("org_xyz").unwrap());
        let retry_after = Some(Duration::from_secs(5));

        let err = decode_error_body(
            StatusCode::TOO_MANY_REQUESTS,
            body,
            request_id,
            organization_id,
            retry_after,
        );

        match err {
            Error::Api(ApiError {
                status,
                body: error_body,
                request_id: rid,
                organization_id: oid,
                retry_after: ra,
            }) => {
                assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
                assert_eq!(error_body.kind, ErrorType::RateLimitError);
                assert_eq!(error_body.message, "slow down");
                assert_eq!(rid.as_ref().map(RequestId::as_str), Some("req_abc"));
                assert_eq!(oid.as_ref().map(OrganizationId::as_str), Some("org_xyz"));
                assert_eq!(ra, Some(Duration::from_secs(5)));
            }
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[test]
    fn decode_error_body_garbage_body_produces_undecodable_api_error() {
        use crate::types::RequestId;
        use http::StatusCode;

        let body = b"this is not json at all";
        let request_id = Some(RequestId::new("req_123").unwrap());

        let err = decode_error_body(
            StatusCode::INTERNAL_SERVER_ERROR,
            body,
            request_id,
            None,
            None,
        );

        match err {
            Error::UndecodableApiError {
                status,
                detail,
                request_id: rid,
            } => {
                assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
                assert_eq!(detail, "this is not json at all");
                assert_eq!(rid.as_ref().map(RequestId::as_str), Some("req_123"));
            }
            other => panic!("expected Error::UndecodableApiError, got {other:?}"),
        }
    }

    #[test]
    fn parse_retry_after_parses_integer_seconds() {
        let d = parse_retry_after("30").unwrap();
        assert_eq!(d, Duration::from_secs(30));
    }

    #[test]
    fn parse_retry_after_returns_none_for_non_integer() {
        assert!(parse_retry_after("not-a-number").is_none());
        assert!(parse_retry_after("").is_none());
    }

    #[test]
    fn decode_api_error_from_parts_extracts_headers() {
        use http::StatusCode;

        let mut headers = http::HeaderMap::new();
        headers.insert(h::REQUEST_ID, "req_xyz".parse().unwrap());
        headers.insert(h::ANTHROPIC_ORG_ID, "org_abc".parse().unwrap());
        headers.insert(h::RETRY_AFTER, "10".parse().unwrap());

        let body =
            br#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#;

        let err = decode_api_error_from_parts(StatusCode::SERVICE_UNAVAILABLE, &headers, body);

        match err {
            Error::Api(ApiError {
                request_id,
                organization_id,
                retry_after,
                ..
            }) => {
                assert_eq!(request_id.as_ref().map(RequestId::as_str), Some("req_xyz"));
                assert_eq!(
                    organization_id.as_ref().map(OrganizationId::as_str),
                    Some("org_abc")
                );
                assert_eq!(retry_after, Some(Duration::from_secs(10)));
            }
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }
}
