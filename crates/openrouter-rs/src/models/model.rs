//! Wire-format DTOs for a single model entry in the models-catalog response.
//!
//! Exists as its own file so the catalog data structures are separate from the
//! resource dispatch logic in `resource.rs`.
//!
//! Responsibilities:
//! - [`Model`] — the v0 subset of a single model entry (id, name,
//!   `context_length`, pricing).
//! - [`Pricing`] — the complete per-token pricing object for a model, with
//!   required `prompt`/`completion` fields and six optional fields covering
//!   specialized token categories.
//!
//! Not responsible for:
//! - Sending the HTTP request — that is `resource.rs`.
//! - Validating model IDs beyond what `ModelId` enforces.

use crate::types::{ModelId, PricePerToken};

/// Pricing information for a single model in the catalog.
///
/// `prompt` and `completion` are always present; the remaining six fields cover
/// specialized token categories that only some models expose.
///
/// All prices are decimal strings from the wire format; see [`PricePerToken`]
/// for the representation choice.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Pricing {
    /// Price per prompt (input) token.
    pub prompt: PricePerToken,
    /// Price per completion (output) token.
    pub completion: PricePerToken,
    /// Price per token read from the prompt cache, if supported.
    pub input_cache_read: Option<PricePerToken>,
    /// Price per token written to the prompt cache, if supported.
    pub input_cache_write: Option<PricePerToken>,
    /// Price per image token, if supported.
    pub image: Option<PricePerToken>,
    /// Price per web-search operation token, if supported.
    pub web_search: Option<PricePerToken>,
    /// Price per internal-reasoning token, if supported.
    pub internal_reasoning: Option<PricePerToken>,
    /// Price per audio token, if supported.
    pub audio: Option<PricePerToken>,
}

/// A single model entry from the `GET /models` catalog (v0 subset).
///
/// The catalog returns 18 fields per entry; this struct captures the four-field
/// v0 subset. Unknown fields are silently ignored on decode (serde default).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Model {
    /// OpenRouter model identifier (e.g. `anthropic/claude-sonnet-4-5`).
    pub id: ModelId,
    /// Human-readable model name.
    pub name: String,
    /// Maximum context window in tokens.
    pub context_length: u64,
    /// Per-token pricing for this model.
    pub pricing: Pricing,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    const FULL_ENTRY: &str = r#"{
        "id": "anthropic/claude-sonnet-4-5",
        "name": "Anthropic: Claude Sonnet 4.5",
        "context_length": 200000,
        "pricing": {
            "prompt": "0.000003",
            "completion": "0.000015",
            "input_cache_read": "0.0000003",
            "input_cache_write": "0.00000375",
            "image": "0.0048",
            "web_search": "0.001",
            "internal_reasoning": "0.000003",
            "audio": "0.000006"
        },
        "description": "ignored extra field",
        "architecture": {"ignored": true}
    }"#;

    const MINIMAL_ENTRY: &str = r#"{
        "id": "openai/gpt-4o",
        "name": "OpenAI: GPT-4o",
        "context_length": 128000,
        "pricing": {
            "prompt": "0.0000025",
            "completion": "0.00001"
        }
    }"#;

    #[test]
    fn decode_full_entry_with_all_pricing_keys() {
        let model: Model = serde_json::from_str(FULL_ENTRY).unwrap();

        assert_eq!(model.id.as_str(), "anthropic/claude-sonnet-4-5");
        assert_eq!(model.name, "Anthropic: Claude Sonnet 4.5");
        assert_eq!(model.context_length, 200_000);

        let p = &model.pricing;
        assert_eq!(p.prompt.as_str(), "0.000003");
        assert_eq!(p.completion.as_str(), "0.000015");
        assert_eq!(p.input_cache_read.as_ref().unwrap().as_str(), "0.0000003");
        assert_eq!(p.input_cache_write.as_ref().unwrap().as_str(), "0.00000375");
        assert_eq!(p.image.as_ref().unwrap().as_str(), "0.0048");
        assert_eq!(p.web_search.as_ref().unwrap().as_str(), "0.001");
        assert_eq!(p.internal_reasoning.as_ref().unwrap().as_str(), "0.000003");
        assert_eq!(p.audio.as_ref().unwrap().as_str(), "0.000006");
    }

    #[test]
    fn decode_minimal_entry_leaves_optional_pricing_fields_none() {
        let model: Model = serde_json::from_str(MINIMAL_ENTRY).unwrap();

        assert_eq!(model.id.as_str(), "openai/gpt-4o");
        assert_eq!(model.context_length, 128_000);

        let p = &model.pricing;
        assert_eq!(p.prompt.as_str(), "0.0000025");
        assert_eq!(p.completion.as_str(), "0.00001");
        assert!(p.input_cache_read.is_none());
        assert!(p.input_cache_write.is_none());
        assert!(p.image.is_none());
        assert!(p.web_search.is_none());
        assert!(p.internal_reasoning.is_none());
        assert!(p.audio.is_none());
    }

    #[test]
    fn unknown_top_level_fields_are_ignored() {
        // The full entry has `description` and `architecture` — both unknown to
        // this struct. Deserialization must succeed rather than error.
        let result = serde_json::from_str::<Model>(FULL_ENTRY);
        assert!(result.is_ok());
    }
}
