//! Token-accounting figures returned alongside a chat completion.
//!
//! Exists separately from the rest of the response because cache-accounting
//! fields attach here independently of the completion envelope.
//!
//! Responsibilities:
//! - [`Usage`] — prompt / completion / total token counts and optional cost.

use serde::Deserialize;

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
    fn decodes_with_cost() {
        let u: Usage = serde_json::from_value(json!({
            "prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3, "cost": 0.5
        }))
        .unwrap();
        assert_eq!(u.cost, Some(0.5));
    }
}
