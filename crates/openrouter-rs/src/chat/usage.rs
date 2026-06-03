//! Token-accounting figures returned alongside a chat completion.
//!
//! Exists separately from the rest of the response because cache-accounting
//! fields attach here independently of the completion envelope.
//!
//! Responsibilities:
//! - [`Usage`] — prompt / completion / total token counts, optional cost, and
//!   the optional cache/token/cost breakdown carriers.

use serde::Deserialize;

use crate::chat::token_details::{CompletionTokensDetails, CostDetails, PromptTokensDetails};

/// Token counts (and optional cost) for one chat completion.
///
/// Counts are server-reported. `cost` is the credit cost when the gateway
/// includes it; it is absent otherwise.
///
/// # Examples
/// ```
/// use openrouter_rs::chat::Usage;
/// let u: Usage = serde_json::from_value(serde_json::json!({
///     "prompt_tokens": 12, "completion_tokens": 8, "total_tokens": 20
/// })).unwrap();
/// assert_eq!(u.total_tokens, 20);
/// assert!(u.cost.is_none());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct Usage {
    /// Tokens consumed by the prompt.
    pub prompt_tokens: u32,
    /// Tokens produced in the completion.
    pub completion_tokens: u32,
    /// Total tokens (`prompt_tokens + completion_tokens`).
    pub total_tokens: u32,
    /// Credit cost of the request, when reported.
    #[serde(default)]
    pub cost: Option<f64>,
    /// Cost saved by the prompt cache; negative on cache writes, positive on
    /// reads. Reported by the prompt-caching layer; may be absent.
    #[serde(default)]
    pub cache_discount: Option<f64>,
    /// Upstream inference cost breakdown, when reported.
    #[serde(default)]
    pub cost_details: Option<CostDetails>,
    /// Whether the request used a bring-your-own-key provider.
    #[serde(default)]
    pub is_byok: Option<bool>,
    /// Input-side token breakdown (including prompt-cache stats), when reported.
    #[serde(default)]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    /// Output-side token breakdown, when reported.
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokensDetails>,
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
    fn decodes_without_cost() {
        let u: Usage = serde_json::from_value(
            json!({ "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3 }),
        )
        .unwrap();
        assert_eq!(
            (u.prompt_tokens, u.completion_tokens, u.total_tokens),
            (1, 2, 3)
        );
        assert_eq!(u.cost, None);
    }

    #[test]
    fn decodes_full_cache_and_token_details() {
        let u: Usage = serde_json::from_value(json!({
            "prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120,
            "cost": 0.002, "cache_discount": 0.0008, "is_byok": false,
            "cost_details": { "upstream_inference_prompt_cost": 0.001 },
            "prompt_tokens_details": { "cached_tokens": 80, "cache_write_tokens": 0 },
            "completion_tokens_details": { "reasoning_tokens": 5 }
        }))
        .unwrap();
        assert_eq!(u.cache_discount, Some(0.0008));
        assert_eq!(u.is_byok, Some(false));
        assert_eq!(u.prompt_tokens_details.unwrap().cached_tokens, Some(80));
        assert_eq!(
            u.completion_tokens_details.unwrap().reasoning_tokens,
            Some(5)
        );
        assert_eq!(
            u.cost_details.unwrap().upstream_inference_prompt_cost,
            Some(0.001)
        );
    }

    #[test]
    fn baseline_usage_still_decodes_with_new_fields_absent() {
        let u: Usage = serde_json::from_value(
            json!({ "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3 }),
        )
        .unwrap();
        assert_eq!(u.cache_discount, None);
        assert_eq!(u.is_byok, None);
        assert!(u.prompt_tokens_details.is_none());
        assert!(u.completion_tokens_details.is_none());
        assert!(u.cost_details.is_none());
    }

    #[test]
    fn decodes_with_cost() {
        let u: Usage = serde_json::from_value(json!({
            "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3, "cost": 0.5
        }))
        .unwrap();
        assert_eq!(u.cost, Some(0.5));
    }
}
