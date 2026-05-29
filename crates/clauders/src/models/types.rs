//! Response types for the Models resource.
//!
//! Exists as a separate module so the wire-format types are decoupled from
//! the resource dispatch logic in `resource.rs`.
//!
//! Responsibilities:
//! - Define [`ModelInfo`], the per-model record returned by both `list` and
//!   `get` endpoints.
//! - Define [`ModelInfoKind`], the discriminant enum (currently only
//!   `"model"`).
//! - Define [`ModelList`], the paginated list wrapper returned by
//!   `GET /v1/models`.

use crate::types::ModelId;

/// The kind of object returned in a models list entry.
///
/// Currently the API always returns `"model"`. Additional variants may
/// appear in the future.
///
/// # Examples
///
/// ```
/// use clauders::models::ModelInfoKind;
///
/// let k: ModelInfoKind = serde_json::from_str(r#""model""#).unwrap();
/// assert_eq!(k, ModelInfoKind::Model);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelInfoKind {
    /// A Claude model.
    Model,
}

/// Metadata record for a single Claude model.
///
/// # Examples
///
/// ```
/// use clauders::models::ModelInfo;
///
/// let json = r#"{
///     "id": "claude-sonnet-4-5",
///     "display_name": "Claude Sonnet 4.5",
///     "created_at": "2025-09-01T00:00:00Z",
///     "type": "model"
/// }"#;
/// let info: ModelInfo = serde_json::from_str(json).unwrap();
/// assert_eq!(info.display_name, "Claude Sonnet 4.5");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ModelInfo {
    /// Unique model identifier.
    pub id: ModelId,
    /// Human-readable model name.
    pub display_name: String,
    /// ISO 8601 creation timestamp, kept as a `String` for forward
    /// compatibility with format variations.
    pub created_at: String,
    /// Object kind; always `"model"` in current API responses.
    #[serde(rename = "type")]
    pub kind: ModelInfoKind,
}

/// Paginated list of [`ModelInfo`] records returned by `GET /v1/models`.
///
/// # Examples
///
/// ```
/// use clauders::models::ModelList;
///
/// let json = r#"{
///     "data": [],
///     "has_more": false,
///     "first_id": null,
///     "last_id": null
/// }"#;
/// let list: ModelList = serde_json::from_str(json).unwrap();
/// assert!(!list.has_more);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct ModelList {
    /// Model records on this page.
    pub data: Vec<ModelInfo>,
    /// Whether additional pages exist.
    pub has_more: bool,
    /// ID of the first record in `data`, if any.
    pub first_id: Option<ModelId>,
    /// ID of the last record in `data`, if any.
    pub last_id: Option<ModelId>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn model_info_kind_deserializes_from_model_string() {
        let k: ModelInfoKind = serde_json::from_str(r#""model""#).unwrap();
        assert_eq!(k, ModelInfoKind::Model);
    }

    #[test]
    fn model_info_deserializes_full_record() {
        let json = r#"{
            "id": "claude-opus-4-7",
            "display_name": "Claude Opus 4.7",
            "created_at": "2026-01-01T00:00:00Z",
            "type": "model"
        }"#;

        let info: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.display_name, "Claude Opus 4.7");
        assert_eq!(info.created_at, "2026-01-01T00:00:00Z");
        assert_eq!(info.kind, ModelInfoKind::Model);
    }

    #[test]
    fn model_list_deserializes_with_optional_ids_null() {
        let json = r#"{
            "data": [],
            "has_more": false,
            "first_id": null,
            "last_id": null
        }"#;

        let list: ModelList = serde_json::from_str(json).unwrap();
        assert!(list.data.is_empty());
        assert!(!list.has_more);
        assert!(list.first_id.is_none());
        assert!(list.last_id.is_none());
    }

    #[test]
    fn model_list_deserializes_with_ids_present() {
        let json = r#"{
            "data": [
                {
                    "id": "claude-sonnet-4-5",
                    "display_name": "Claude Sonnet 4.5",
                    "created_at": "2025-09-01T00:00:00Z",
                    "type": "model"
                }
            ],
            "has_more": false,
            "first_id": "claude-sonnet-4-5",
            "last_id": "claude-sonnet-4-5"
        }"#;

        let list: ModelList = serde_json::from_str(json).unwrap();
        assert_eq!(list.data.len(), 1);
        assert!(list.first_id.is_some());
        assert!(list.last_id.is_some());
    }
}
