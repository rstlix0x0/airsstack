//! Per-model token and cost breakdown from a result frame.

use serde::{Deserialize, Serialize};

/// One model's usage/cost breakdown, the value type of a result frame's
/// `modelUsage` map (keyed by model id).
///
/// The wire object uses camelCase keys; the typed numerics below are the
/// stable core. Forward-compatible extras (`canonicalModel`, `provider`, and
/// any future key) are preserved in [`ModelUsage::extra`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelUsage {
    /// Input tokens consumed by this model.
    #[serde(rename = "inputTokens", default)]
    pub input_tokens: u64,
    /// Output tokens produced by this model.
    #[serde(rename = "outputTokens", default)]
    pub output_tokens: u64,
    /// Input tokens served from the prompt cache.
    #[serde(rename = "cacheReadInputTokens", default)]
    pub cache_read_input_tokens: u64,
    /// Input tokens written to the prompt cache.
    #[serde(rename = "cacheCreationInputTokens", default)]
    pub cache_creation_input_tokens: u64,
    /// Web-search tool requests attributed to this model.
    #[serde(rename = "webSearchRequests", default)]
    pub web_search_requests: u64,
    /// Estimated cost in USD for this model's usage.
    #[serde(rename = "costUSD", default)]
    pub cost_usd: f64,
    /// The model's context-window size in tokens.
    #[serde(rename = "contextWindow", default)]
    pub context_window: u64,
    /// The model's maximum output tokens.
    #[serde(rename = "maxOutputTokens", default)]
    pub max_output_tokens: u64,
    /// Forward-compatible extras (`canonicalModel`, `provider`, …).
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[cfg(test)]
mod tests {
    #![expect(clippy::expect_used, reason = "test assertions use expect for context")]

    use super::ModelUsage;

    #[test]
    fn decodes_captured_model_usage_value() {
        // captured: claude -p "say hi" --output-format stream-json --verbose
        let json = r#"{"inputTokens":2,"outputTokens":6,"cacheReadInputTokens":0,
          "cacheCreationInputTokens":25595,"webSearchRequests":0,"costUSD":0.25611,
          "contextWindow":1000000,"maxOutputTokens":64000,
          "canonicalModel":"claude-opus-4-8","provider":"firstParty"}"#;
        let u: ModelUsage = serde_json::from_str(json).expect("decode");
        assert_eq!(u.input_tokens, 2);
        assert_eq!(u.output_tokens, 6);
        assert_eq!(u.cache_creation_input_tokens, 25595);
        assert_eq!(u.context_window, 1_000_000);
        assert_eq!(u.max_output_tokens, 64000);
        // strings the numeric core does not type survive in extra
        assert_eq!(
            u.extra.get("canonicalModel").and_then(|v| v.as_str()),
            Some("claude-opus-4-8")
        );
        assert_eq!(
            u.extra.get("provider").and_then(|v| v.as_str()),
            Some("firstParty")
        );
    }
}
