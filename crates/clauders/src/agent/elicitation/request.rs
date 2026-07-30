//! The inbound elicitation request handed to an [`ElicitationPolicy`].

use serde::Deserialize;

/// Which elicitation form the MCP server requested.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElicitationMode {
    /// Structured input validated against `requested_schema`.
    #[default]
    Form,
    /// The user must visit `url` (OAuth-style authorization).
    Url,
    /// A mode this version does not model.
    #[serde(other)]
    Unknown,
}

/// A structured-input request an MCP server raised mid-tool-call.
///
/// Forwarded by the binary as an `elicitation` control request and handed to
/// the registered [`crate::agent::ElicitationPolicy`]. The
/// `title`/`display_name`/`description` trio mirrors the permission request's
/// context fields.
///
/// This type is `#[non_exhaustive]`, so downstream crates build it through
/// [`Default::default`] and then set the fields they need — functional-update
/// syntax (`..Default::default()`) is not available across the crate
/// boundary:
///
/// ```
/// use clauders::agent::ElicitationRequest;
///
/// let mut request = ElicitationRequest::default();
/// request.message = "Pick a branch".to_string();
///
/// assert_eq!(request.message, "Pick a branch");
/// ```
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct ElicitationRequest {
    /// Correlation id the binary assigned this elicitation, when it sent one.
    pub elicitation_id: Option<String>,
    /// Human-readable prompt to present.
    pub message: String,
    /// Whether this is a form-mode or url-mode elicitation.
    ///
    /// `None` when the binary did not say; treat it as undetermined rather
    /// than assuming a form. <code>Some([ElicitationMode::Unknown])</code>
    /// when the binary sent a mode this version does not model; decline
    /// such a request rather than guessing.
    pub mode: Option<ElicitationMode>,
    /// The requested MCP JSON Schema. Present for [`ElicitationMode::Form`].
    pub requested_schema: Option<serde_json::Value>,
    /// The URL the user must visit. Present for [`ElicitationMode::Url`].
    pub url: Option<String>,
    /// The MCP server that raised the elicitation.
    pub mcp_server_name: String,
    /// Short human title.
    pub title: Option<String>,
    /// Display name of the requesting server.
    pub display_name: Option<String>,
    /// Longer human description.
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::{ElicitationMode, ElicitationRequest};

    #[test]
    fn mode_deserializes_snake_case_wire_names() {
        let form: ElicitationMode = serde_json::from_str("\"form\"").expect("form");
        assert_eq!(form, ElicitationMode::Form);
        let url: ElicitationMode = serde_json::from_str("\"url\"").expect("url");
        assert_eq!(url, ElicitationMode::Url);
    }

    #[test]
    fn mode_falls_back_to_unknown_for_unrecognized_wire_name() {
        let mode: ElicitationMode = serde_json::from_str("\"nope\"").expect("decode");
        assert_eq!(mode, ElicitationMode::Unknown);
    }

    #[test]
    fn request_carries_form_mode_fields() {
        let request = ElicitationRequest {
            elicitation_id: Some("elic_1".to_string()),
            message: "Pick a branch".to_string(),
            mode: Some(ElicitationMode::Form),
            requested_schema: Some(serde_json::json!({ "type": "object" })),
            url: None,
            mcp_server_name: "git".to_string(),
            title: Some("Branch".to_string()),
            display_name: Some("Git".to_string()),
            description: Some("Choose a branch".to_string()),
        };
        assert_eq!(request.elicitation_id.as_deref(), Some("elic_1"));
        assert_eq!(request.mode, Some(ElicitationMode::Form));
        assert_eq!(request.mcp_server_name, "git");
    }

    #[test]
    fn request_carries_url_mode_fields() {
        let request = ElicitationRequest {
            elicitation_id: Some("elic_2".to_string()),
            message: "Authorize".to_string(),
            mode: Some(ElicitationMode::Url),
            requested_schema: None,
            url: Some("https://example.com/authorize".to_string()),
            mcp_server_name: "oauth".to_string(),
            title: None,
            display_name: None,
            description: None,
        };
        assert_eq!(request.mode, Some(ElicitationMode::Url));
        assert_eq!(
            request.url.as_deref(),
            Some("https://example.com/authorize")
        );
        assert!(request.requested_schema.is_none());
    }

    #[test]
    fn default_request_is_constructible_downstream() {
        let mut request = ElicitationRequest::default();
        assert!(request.mode.is_none());
        assert!(request.elicitation_id.is_none());
        assert!(request.message.is_empty());
        assert!(request.requested_schema.is_none());
        assert!(request.url.is_none());
        assert!(request.mcp_server_name.is_empty());
        assert!(request.title.is_none());
        assert!(request.display_name.is_none());
        assert!(request.description.is_none());

        request.elicitation_id = Some("elic_3".to_string());
        request.message = "Pick".to_string();

        assert_eq!(request.elicitation_id.as_deref(), Some("elic_3"));
        assert_eq!(request.message, "Pick");
    }
}
