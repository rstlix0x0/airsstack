//! Per-direction token breakdowns and upstream cost breakdown nested in
//! [`crate::chat::Usage`].
//!
//! Exists as its own file so the nested detail structs do not crowd the
//! top-level `Usage` carrier in `usage.rs`. All fields are deserialize-only and
//! optional; an absent field decodes to `None` so a server that omits a
//! breakdown never breaks decode.
//!
//! Responsibilities:
//! - [`PromptTokensDetails`] — input-side token breakdown (cache reads/writes,
//!   audio, video).
//! - [`CompletionTokensDetails`] — output-side token breakdown (reasoning,
//!   audio, prediction accept/reject).
//! - [`CostDetails`] — upstream inference cost breakdown.

use serde::Deserialize;

/// Input-side token breakdown.
///
/// The two cache fields are the prompt-cache stats: `cached_tokens` counts
/// cache **reads**, `cache_write_tokens` counts **writes** on the request that
/// established the entry.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::PromptTokensDetails;
/// let d: PromptTokensDetails = serde_json::from_value(serde_json::json!({
///     "cached_tokens": 1024, "cache_write_tokens": 0
/// })).unwrap();
/// assert_eq!(d.cached_tokens, Some(1024));
/// assert_eq!(d.audio_tokens, None);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct PromptTokensDetails {
    /// Tokens served from the prompt cache (cache reads).
    #[serde(default)]
    pub cached_tokens: Option<u32>,
    /// Tokens written to the prompt cache on first establishment (cache writes).
    #[serde(default)]
    pub cache_write_tokens: Option<u32>,
    /// Input audio tokens.
    #[serde(default)]
    pub audio_tokens: Option<u32>,
    /// Input video tokens.
    #[serde(default)]
    pub video_tokens: Option<u32>,
}

/// Output-side token breakdown.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::CompletionTokensDetails;
/// let d: CompletionTokensDetails = serde_json::from_value(serde_json::json!({
///     "reasoning_tokens": 50
/// })).unwrap();
/// assert_eq!(d.reasoning_tokens, Some(50));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct CompletionTokensDetails {
    /// Tokens spent on hidden reasoning (reasoning models).
    #[serde(default)]
    pub reasoning_tokens: Option<u32>,
    /// Output audio tokens.
    #[serde(default)]
    pub audio_tokens: Option<u32>,
    /// Predicted-output tokens that were accepted.
    #[serde(default)]
    pub accepted_prediction_tokens: Option<u32>,
    /// Predicted-output tokens that were rejected.
    #[serde(default)]
    pub rejected_prediction_tokens: Option<u32>,
}

/// Upstream inference cost breakdown.
///
/// All values are USD; absent fields decode to `None`.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::CostDetails;
/// let d: CostDetails = serde_json::from_value(serde_json::json!({
///     "upstream_inference_prompt_cost": 0.001
/// })).unwrap();
/// assert_eq!(d.upstream_inference_prompt_cost, Some(0.001));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
pub struct CostDetails {
    /// Total upstream inference cost.
    #[serde(default)]
    pub upstream_inference_cost: Option<f64>,
    /// Upstream prompt-side inference cost.
    #[serde(default)]
    pub upstream_inference_prompt_cost: Option<f64>,
    /// Upstream completion-side inference cost.
    #[serde(default)]
    pub upstream_inference_completions_cost: Option<f64>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;
    use serde_json::json;

    #[test]
    fn prompt_details_decodes_cache_fields_and_defaults_rest() {
        let d: PromptTokensDetails =
            serde_json::from_value(json!({ "cached_tokens": 1024, "cache_write_tokens": 16 }))
                .unwrap();
        assert_eq!(d.cached_tokens, Some(1024));
        assert_eq!(d.cache_write_tokens, Some(16));
        assert_eq!(d.audio_tokens, None);
        assert_eq!(d.video_tokens, None);
    }

    #[test]
    fn prompt_details_decodes_all_fields() {
        let d: PromptTokensDetails = serde_json::from_value(json!({
            "cached_tokens": 1, "cache_write_tokens": 2, "audio_tokens": 3, "video_tokens": 4
        }))
        .unwrap();
        assert_eq!(
            (
                d.cached_tokens,
                d.cache_write_tokens,
                d.audio_tokens,
                d.video_tokens
            ),
            (Some(1), Some(2), Some(3), Some(4))
        );
    }

    #[test]
    fn empty_object_decodes_to_all_none() {
        let d: PromptTokensDetails = serde_json::from_value(json!({})).unwrap();
        assert_eq!(d, PromptTokensDetails::default());
        let c: CompletionTokensDetails = serde_json::from_value(json!({})).unwrap();
        assert_eq!(c, CompletionTokensDetails::default());
    }

    #[test]
    fn completion_details_decodes_all_fields() {
        let d: CompletionTokensDetails = serde_json::from_value(json!({
            "reasoning_tokens": 9, "audio_tokens": 8,
            "accepted_prediction_tokens": 7, "rejected_prediction_tokens": 6
        }))
        .unwrap();
        assert_eq!(d.reasoning_tokens, Some(9));
        assert_eq!(d.audio_tokens, Some(8));
        assert_eq!(d.accepted_prediction_tokens, Some(7));
        assert_eq!(d.rejected_prediction_tokens, Some(6));
    }

    #[test]
    fn cost_details_decodes_and_tolerates_missing() {
        let d: CostDetails =
            serde_json::from_value(json!({ "upstream_inference_prompt_cost": 0.5 })).unwrap();
        assert_eq!(d.upstream_inference_prompt_cost, Some(0.5));
        assert_eq!(d.upstream_inference_cost, None);
        assert_eq!(d.upstream_inference_completions_cost, None);
    }
}
