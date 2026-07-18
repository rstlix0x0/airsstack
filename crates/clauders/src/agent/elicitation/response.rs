//! The outcome an [`crate::agent::ElicitationPolicy`] returns.

/// The outcome of an elicitation, returned by the registered policy.
///
/// Serializes to the binary's `{ "action": ..., "content": ... }` control
/// response payload. `content` is present only for [`Self::Accept`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElicitationResponse {
    /// The user supplied values conforming to the requested schema.
    Accept(serde_json::Value),
    /// The user actively declined to supply input.
    Decline,
    /// The elicitation was aborted (dismissed, timed out, or cancelled).
    Cancel,
}

impl ElicitationResponse {
    /// Lower the outcome to its control-response payload.
    #[must_use]
    pub fn into_response_value(self) -> serde_json::Value {
        match self {
            Self::Accept(content) => {
                serde_json::json!({ "action": "accept", "content": content })
            }
            Self::Decline => serde_json::json!({ "action": "decline" }),
            Self::Cancel => serde_json::json!({ "action": "cancel" }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ElicitationResponse;

    #[test]
    fn accept_carries_action_and_content() {
        let value = ElicitationResponse::Accept(serde_json::json!({ "branch": "main" }))
            .into_response_value();
        assert_eq!(value["action"], "accept");
        assert_eq!(value["content"]["branch"], "main");
    }

    #[test]
    fn decline_carries_action_only() {
        let value = ElicitationResponse::Decline.into_response_value();
        assert_eq!(value["action"], "decline");
        assert!(value.get("content").is_none());
    }

    #[test]
    fn cancel_carries_action_only() {
        let value = ElicitationResponse::Cancel.into_response_value();
        assert_eq!(value["action"], "cancel");
        assert!(value.get("content").is_none());
    }
}
