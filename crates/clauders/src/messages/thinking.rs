//! Extended-thinking configuration for the Messages API.
//!
//! Exists as its own module so the three-variant thinking union is scoped
//! separately from the rest of the request surface.
//!
//! Responsibilities:
//! - Define [`ThinkingConfig`], the `thinking` request parameter.
//! - Define [`ThinkingDisplay`], how thinking content appears in the response.
//!
//! Not responsible for:
//! - Thinking *output* — that is [`crate::messages::ThinkingBlock`] in
//!   `content.rs`.
//! - Sending the request — that is `resource.rs`.

/// Configuration for the model's extended thinking.
///
/// This is a request-only type: it is serialized into the `thinking` field
/// of `POST /v1/messages` and never decoded from a response, so it
/// implements [`serde::Serialize`] only.
///
/// Construct it with one of the five constructors rather than by writing a
/// variant literal. There is deliberately no `with_display` method: two of
/// the three variants accept a `display` and [`ThinkingConfig::Disabled`]
/// does not, so a chainer would have to silently do nothing on that variant.
///
/// # Examples
///
/// ```
/// use clauders::messages::{ThinkingConfig, ThinkingDisplay};
///
/// let cfg = ThinkingConfig::enabled_with_display(2048, ThinkingDisplay::Omitted);
/// let j = serde_json::to_value(&cfg).unwrap();
/// assert_eq!(j["type"], "enabled");
/// assert_eq!(j["budget_tokens"], 2048);
/// assert_eq!(j["display"], "omitted");
/// ```
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThinkingConfig {
    /// The model decides how much to think.
    Adaptive {
        /// How thinking appears in the response. `None` leaves the API
        /// default (`summarized`) in force.
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<ThinkingDisplay>,
    },
    /// Thinking is turned off.
    ///
    /// Not supported by every model; the API rejects it where it does not
    /// apply.
    Disabled,
    /// Thinking is on with an explicit token budget.
    Enabled {
        /// Tokens the model may spend on internal reasoning.
        ///
        /// The API requires this to be at least 1024 and less than the
        /// request's `max_tokens`. Neither bound is checked here — both are
        /// enforced by the server, which returns an `invalid_request_error`.
        budget_tokens: u32,
        /// How thinking appears in the response. `None` leaves the API
        /// default (`summarized`) in force.
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<ThinkingDisplay>,
    },
}

impl ThinkingConfig {
    /// Let the model decide how much to think, with the default display.
    #[must_use]
    pub const fn adaptive() -> Self {
        Self::Adaptive { display: None }
    }

    /// Let the model decide how much to think, with an explicit display.
    #[must_use]
    pub const fn adaptive_with_display(display: ThinkingDisplay) -> Self {
        Self::Adaptive {
            display: Some(display),
        }
    }

    /// Turn thinking off.
    #[must_use]
    pub const fn disabled() -> Self {
        Self::Disabled
    }

    /// Turn thinking on with an explicit token budget and the default
    /// display.
    #[must_use]
    pub const fn enabled(budget_tokens: u32) -> Self {
        Self::Enabled {
            budget_tokens,
            display: None,
        }
    }

    /// Turn thinking on with an explicit token budget and display.
    #[must_use]
    pub const fn enabled_with_display(budget_tokens: u32, display: ThinkingDisplay) -> Self {
        Self::Enabled {
            budget_tokens,
            display: Some(display),
        }
    }
}

/// How thinking content appears in the response.
///
/// The API default is [`ThinkingDisplay::Summarized`] on every model; set
/// this only to change that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingDisplay {
    /// Thinking is returned normally.
    Summarized,
    /// Thinking is redacted, but a signature is returned so a multi-turn
    /// conversation can continue.
    Omitted,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::{ThinkingConfig, ThinkingDisplay};

    #[test]
    fn adaptive_without_display_omits_the_key() {
        let j = serde_json::to_value(ThinkingConfig::adaptive()).unwrap();
        assert_eq!(j, serde_json::json!({"type": "adaptive"}));
        assert!(
            j.get("display").is_none(),
            "display must be absent when unset, not null"
        );
    }

    #[test]
    fn adaptive_with_display_carries_it() {
        let j = serde_json::to_value(ThinkingConfig::adaptive_with_display(
            ThinkingDisplay::Omitted,
        ))
        .unwrap();
        assert_eq!(
            j,
            serde_json::json!({"type": "adaptive", "display": "omitted"})
        );
    }

    #[test]
    fn disabled_serializes_to_type_only() {
        let j = serde_json::to_value(ThinkingConfig::disabled()).unwrap();
        assert_eq!(j, serde_json::json!({"type": "disabled"}));
    }

    #[test]
    fn enabled_carries_budget_tokens() {
        let j = serde_json::to_value(ThinkingConfig::enabled(1024)).unwrap();
        assert_eq!(
            j,
            serde_json::json!({"type": "enabled", "budget_tokens": 1024})
        );
    }

    #[test]
    fn enabled_with_display_carries_both() {
        let j = serde_json::to_value(ThinkingConfig::enabled_with_display(
            2048,
            ThinkingDisplay::Summarized,
        ))
        .unwrap();
        assert_eq!(
            j,
            serde_json::json!({
                "type": "enabled",
                "budget_tokens": 2048,
                "display": "summarized"
            })
        );
    }

    #[test]
    fn display_variants_serialize_lowercase() {
        assert_eq!(
            serde_json::to_value(ThinkingDisplay::Summarized).unwrap(),
            serde_json::json!("summarized")
        );
        assert_eq!(
            serde_json::to_value(ThinkingDisplay::Omitted).unwrap(),
            serde_json::json!("omitted")
        );
    }
}
