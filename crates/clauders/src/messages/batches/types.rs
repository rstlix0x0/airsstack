//! Wire types for the Message Batches API.
//!
//! Exists as a separate module so the batch-specific structs are isolated
//! from the core Messages API request and response types.
//!
//! Responsibilities:
//! - Input types: [`BatchRequest`], [`BatchRequestBuilder`],
//!   [`BatchedMessageRequest`].
//! - Status types: [`Batch`], [`BatchKind`], [`BatchStatus`],
//!   [`RequestCounts`], [`BatchList`].
//! - Result types: [`BatchResultRow`], [`BatchResult`],
//!   [`DeletedMessageBatch`], [`DeletedBatchKind`].
//!
//! Not responsible for:
//! - HTTP dispatch — see `resource.rs`.
//! - JSONL result streaming — see `results.rs`.

use crate::error::ApiErrorBody;
use crate::messages::request::MessageRequest;
use crate::messages::response::Message;
use crate::types::{BatchId, CustomRequestId};

/// Input to `POST /v1/messages/batches`.
///
/// Use [`BatchRequest::builder`] to construct an instance.
///
/// # Examples
///
/// ```
/// # use clauders::messages::{BatchRequest, MessageRequest};
/// # use clauders::types::{BatchId, CustomRequestId, MaxTokens, ModelId};
/// let req = BatchRequest::builder()
///     .add(
///         CustomRequestId::new("r1").unwrap(),
///         MessageRequest::builder()
///             .model(ModelId::claude_sonnet_4_5())
///             .max_tokens(MaxTokens::new(16).unwrap())
///             .add_user_text("hello")
///             .build(),
///     )
///     .build();
/// assert_eq!(req.requests.len(), 1);
/// ```
#[derive(Clone, Debug, serde::Serialize)]
pub struct BatchRequest {
    /// The individual message requests to submit in this batch.
    pub requests: Vec<BatchedMessageRequest>,
}

impl BatchRequest {
    /// Return a builder for constructing a [`BatchRequest`].
    #[must_use]
    pub fn builder() -> BatchRequestBuilder {
        BatchRequestBuilder::default()
    }
}

/// Builder for [`BatchRequest`].
#[derive(Clone, Debug, Default)]
pub struct BatchRequestBuilder {
    requests: Vec<BatchedMessageRequest>,
}

impl BatchRequestBuilder {
    /// Append a single row to the batch.
    #[must_use]
    pub fn add(mut self, custom_id: CustomRequestId, params: MessageRequest) -> Self {
        self.requests
            .push(BatchedMessageRequest { custom_id, params });
        self
    }

    /// Append multiple rows to the batch.
    #[must_use]
    pub fn add_many(mut self, items: impl IntoIterator<Item = BatchedMessageRequest>) -> Self {
        self.requests.extend(items);
        self
    }

    /// Consume the builder and return the [`BatchRequest`].
    #[must_use]
    pub fn build(self) -> BatchRequest {
        BatchRequest {
            requests: self.requests,
        }
    }
}

/// One row inside a [`BatchRequest`].
#[derive(Clone, Debug, serde::Serialize)]
pub struct BatchedMessageRequest {
    /// Caller-supplied identifier for correlating this row with its result.
    pub custom_id: CustomRequestId,
    /// Full message request parameters for this row.
    pub params: MessageRequest,
}

/// Batch object returned by create, get, list, and cancel endpoints.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Batch {
    /// Server-generated batch identifier.
    pub id: BatchId,
    /// Discriminator field; always [`BatchKind::MessageBatch`].
    #[serde(rename = "type")]
    pub kind: BatchKind,
    /// Current processing status of this batch.
    pub processing_status: BatchStatus,
    /// Counts of requests in each terminal and in-progress state.
    pub request_counts: RequestCounts,
    /// When the batch finished processing (RFC-3339), if it has ended.
    pub ended_at: Option<String>,
    /// When the batch was created (RFC-3339).
    pub created_at: String,
    /// When the batch will expire if not completed (RFC-3339).
    pub expires_at: String,
    /// When the batch was archived (RFC-3339), if it has been archived.
    pub archived_at: Option<String>,
    /// When a cancel was initiated (RFC-3339), if cancellation was requested.
    pub cancel_initiated_at: Option<String>,
    /// URL from which results can be streamed once the batch has ended.
    pub results_url: Option<String>,
}

/// Discriminator for the batch object type field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchKind {
    /// Standard message batch.
    MessageBatch,
}

/// Processing status of a batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Requests are still being processed.
    InProgress,
    /// A cancellation request has been issued; processing is winding down.
    Canceling,
    /// All requests have finished (succeeded, errored, canceled, or expired).
    Ended,
}

/// Counts of requests in each final or intermediate state.
#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
pub struct RequestCounts {
    /// Requests still being processed.
    #[serde(default)]
    pub processing: u32,
    /// Requests that completed successfully.
    #[serde(default)]
    pub succeeded: u32,
    /// Requests that encountered an API error.
    #[serde(default)]
    pub errored: u32,
    /// Requests that were canceled before processing.
    #[serde(default)]
    pub canceled: u32,
    /// Requests that expired before the batch ended.
    #[serde(default)]
    pub expired: u32,
}

/// Paginated list of batches returned by `GET /v1/messages/batches`.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct BatchList {
    /// Batches on this page, most-recent first.
    pub data: Vec<Batch>,
    /// Whether additional pages are available.
    pub has_more: bool,
    /// Identifier of the first batch on this page, for cursor-based paging.
    pub first_id: Option<BatchId>,
    /// Identifier of the last batch on this page, for requesting the next page.
    pub last_id: Option<BatchId>,
}

/// One row in the JSONL results file for an ended batch.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct BatchResultRow {
    /// The caller-supplied identifier used when the request was submitted.
    pub custom_id: CustomRequestId,
    /// The outcome of this row's request.
    pub result: BatchResult,
}

/// Outcome of a single request inside a batch.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BatchResult {
    /// The request completed successfully; `message` holds the response.
    Succeeded {
        /// Decoded message response.
        message: Message,
    },
    /// The request failed with an API error.
    Errored {
        /// Decoded API error body.
        error: ApiErrorBody,
    },
    /// The request was canceled before processing began.
    Canceled,
    /// The request expired before the batch ended.
    Expired,
}

/// Response body from `DELETE /v1/messages/batches/{id}`.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct DeletedMessageBatch {
    /// Identifier of the deleted batch.
    pub id: BatchId,
    /// Discriminator confirming this is a deletion response.
    #[serde(rename = "type")]
    pub kind: DeletedBatchKind,
}

/// Discriminator for the deletion response type field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletedBatchKind {
    /// Confirms the resource was a message batch.
    MessageBatchDeleted,
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

    // ── BatchRequestBuilder ─────────────────────────────────────────────────

    fn minimal_req() -> MessageRequest {
        use crate::types::{MaxTokens, ModelId};
        MessageRequest::builder()
            .model(ModelId::claude_sonnet_4_5())
            .max_tokens(MaxTokens::new(8).unwrap())
            .add_user_text("hi")
            .build()
    }

    #[test]
    fn builder_add_appends_row() {
        let id = CustomRequestId::new("r1").unwrap();
        let req = BatchRequest::builder()
            .add(id.clone(), minimal_req())
            .build();
        assert_eq!(req.requests.len(), 1);
        assert_eq!(req.requests[0].custom_id, id);
    }

    #[test]
    fn builder_add_many_appends_multiple_rows() {
        let rows: Vec<BatchedMessageRequest> = (0..3)
            .map(|i| BatchedMessageRequest {
                custom_id: CustomRequestId::new(format!("r{i}")).unwrap(),
                params: minimal_req(),
            })
            .collect();
        let req = BatchRequest::builder().add_many(rows).build();
        assert_eq!(req.requests.len(), 3);
    }

    #[test]
    fn builder_build_empty_is_allowed() {
        let req = BatchRequest::builder().build();
        assert!(req.requests.is_empty());
    }

    // ── BatchRequest serde wire shape ───────────────────────────────────────

    #[test]
    fn batch_request_serializes_with_requests_key() {
        let req = BatchRequest::builder()
            .add(CustomRequestId::new("r1").unwrap(), minimal_req())
            .build();
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("requests").is_some());
        let rows = json["requests"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["custom_id"], "r1");
        assert!(rows[0].get("params").is_some());
    }

    // ── Batch deserialization ───────────────────────────────────────────────

    const BATCH_JSON: &str = r#"{
        "id": "msgbatch_01",
        "type": "message_batch",
        "processing_status": "in_progress",
        "request_counts": {"processing": 2, "succeeded": 0, "errored": 0, "canceled": 0, "expired": 0},
        "ended_at": null,
        "created_at": "2026-05-28T00:00:00Z",
        "expires_at": "2026-05-29T00:00:00Z",
        "archived_at": null,
        "cancel_initiated_at": null,
        "results_url": null
    }"#;

    #[test]
    fn batch_deserializes_from_json() {
        let batch: Batch = serde_json::from_str(BATCH_JSON).unwrap();
        assert_eq!(batch.id.as_str(), "msgbatch_01");
        assert_eq!(batch.kind, BatchKind::MessageBatch);
        assert_eq!(batch.processing_status, BatchStatus::InProgress);
        assert_eq!(batch.request_counts.processing, 2);
        assert!(batch.ended_at.is_none());
        assert!(batch.archived_at.is_none());
    }

    #[test]
    fn batch_deserializes_with_archived_at_populated() {
        let json = BATCH_JSON.replace(
            r#""archived_at": null"#,
            r#""archived_at": "2026-05-30T00:00:00Z""#,
        );
        let batch: Batch = serde_json::from_str(&json).unwrap();
        assert_eq!(batch.archived_at.as_deref(), Some("2026-05-30T00:00:00Z"));
    }

    // ── RequestCounts defaults ──────────────────────────────────────────────

    #[test]
    fn request_counts_defaults_to_zero() {
        let counts: RequestCounts = serde_json::from_str("{}").unwrap();
        assert_eq!(counts.processing, 0);
        assert_eq!(counts.succeeded, 0);
        assert_eq!(counts.errored, 0);
        assert_eq!(counts.canceled, 0);
        assert_eq!(counts.expired, 0);
    }

    // ── BatchList deserialization ───────────────────────────────────────────

    #[test]
    fn batch_list_deserializes() {
        let json = r#"{"data":[],"has_more":false,"first_id":null,"last_id":null}"#;
        let list: BatchList = serde_json::from_str(json).unwrap();
        assert!(!list.has_more);
        assert!(list.data.is_empty());
    }

    // ── BatchResultRow variants ─────────────────────────────────────────────

    #[test]
    fn batch_result_row_canceled_deserializes() {
        let json = r#"{"custom_id":"r1","result":{"type":"canceled"}}"#;
        let row: BatchResultRow = serde_json::from_str(json).unwrap();
        assert_eq!(row.custom_id.as_str(), "r1");
        assert!(matches!(row.result, BatchResult::Canceled));
    }

    #[test]
    fn batch_result_row_expired_deserializes() {
        let json = r#"{"custom_id":"r2","result":{"type":"expired"}}"#;
        let row: BatchResultRow = serde_json::from_str(json).unwrap();
        assert!(matches!(row.result, BatchResult::Expired));
    }

    #[test]
    fn batch_result_row_errored_deserializes() {
        let json = r#"{"custom_id":"r3","result":{"type":"errored","error":{"type":"invalid_request_error","message":"bad input"}}}"#;
        let row: BatchResultRow = serde_json::from_str(json).unwrap();
        match row.result {
            BatchResult::Errored { error } => {
                assert_eq!(error.message, "bad input");
            }
            other => panic!("expected Errored, got {other:?}"),
        }
    }

    #[test]
    fn batch_result_row_succeeded_deserializes() {
        let json = r#"{"custom_id":"r4","result":{"type":"succeeded","message":{"id":"msg_01","type":"message","role":"assistant","model":"claude-sonnet-4-5","content":[{"type":"text","text":"hi"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":1}}}}"#;
        let row: BatchResultRow = serde_json::from_str(json).unwrap();
        match row.result {
            BatchResult::Succeeded { message } => {
                assert_eq!(message.id.as_str(), "msg_01");
            }
            other => panic!("expected Succeeded, got {other:?}"),
        }
    }

    // ── DeletedMessageBatch ─────────────────────────────────────────────────

    #[test]
    fn deleted_message_batch_deserializes() {
        let json = r#"{"id":"msgbatch_01","type":"message_batch_deleted"}"#;
        let deleted: DeletedMessageBatch = serde_json::from_str(json).unwrap();
        assert_eq!(deleted.id.as_str(), "msgbatch_01");
        assert_eq!(deleted.kind, DeletedBatchKind::MessageBatchDeleted);
    }
}
