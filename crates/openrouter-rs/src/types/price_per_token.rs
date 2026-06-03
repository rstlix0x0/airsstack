//! Decimal-string price newtype for per-token pricing in the models catalog.
//!
//! Exists as its own file to enforce non-negative finite decimal constraints
//! at construction, so catalog-decoding code downstream never re-checks the
//! invariant.
//!
//! Responsibilities:
//! - [`PricePerToken`] — a validated, wire-faithful decimal string representing
//!   a per-token price from the models-catalog response.
//! - [`InvalidPricePerToken`] — the error returned when construction fails.
//!
//! Not responsible for request-side pricing — see `types/price.rs` for the
//! `f64`-backed [`crate::types::Price`] used in provider routing limits.

use std::fmt;

/// A per-token price received from the models-catalog endpoint.
///
/// The wire format delivers prices as decimal strings (`"0.0000003"`), which
/// preserves precision that `f64` JSON parsing cannot. This type wraps the
/// raw string, validates that it parses as a non-negative finite decimal at
/// construction, and exposes helpers to retrieve the raw string or convert to
/// `f64` when approximate arithmetic is acceptable.
///
/// Deserializes via `serde(try_from = "String")` so the validating constructor
/// always runs.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::PricePerToken;
///
/// let p = PricePerToken::new("0.0000003").expect("valid decimal");
/// assert_eq!(p.as_str(), "0.0000003");
///
/// let p2 = PricePerToken::new("0").expect("zero is valid");
/// assert_eq!(p2.to_f64(), 0.0_f64);
///
/// assert!(PricePerToken::new("").is_err());
/// assert!(PricePerToken::new("-1").is_err());
/// assert!(PricePerToken::new("not-a-number").is_err());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct PricePerToken(String);

/// Reasons [`PricePerToken::new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidPricePerToken {
    /// Input was empty.
    #[error("price per token must not be empty")]
    Empty,
    /// Input is not a valid decimal number.
    #[error("price per token is not a valid decimal: {0}")]
    NotANumber(String),
    /// Input parsed as a negative number.
    #[error("price per token must be non-negative")]
    Negative,
    /// Input parsed as infinity or NaN.
    #[error("price per token must be a finite number")]
    NonFinite,
}

impl PricePerToken {
    /// Validate and wrap a decimal-string price.
    ///
    /// The string must parse as a finite, non-negative decimal number. The
    /// original string representation is preserved exactly.
    ///
    /// # Errors
    ///
    /// - [`InvalidPricePerToken::Empty`] — `s` is empty.
    /// - [`InvalidPricePerToken::NotANumber`] — `s` does not parse as `f64`.
    /// - [`InvalidPricePerToken::NonFinite`] — parsed value is `NaN` or infinite.
    /// - [`InvalidPricePerToken::Negative`] — parsed value is negative.
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidPricePerToken> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidPricePerToken::Empty);
        }
        let n: f64 = s
            .parse()
            .map_err(|_| InvalidPricePerToken::NotANumber(s.clone()))?;
        if !n.is_finite() {
            return Err(InvalidPricePerToken::NonFinite);
        }
        if n < 0.0 {
            return Err(InvalidPricePerToken::Negative);
        }
        Ok(Self(s))
    }

    /// Borrow the original decimal string exactly as it appears on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Convert to `f64`, losing any precision beyond what `f64` can represent.
    ///
    /// Parsing the inner string cannot fail because the constructor already
    /// validated it.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the constructor validates that the inner
    /// string parses as a finite `f64`, so the parse here is guaranteed to
    /// succeed.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "constructor guarantees inner string is a valid finite f64; parse cannot fail"
    )]
    pub fn to_f64(&self) -> f64 {
        self.0
            .parse()
            .expect("invariant: PricePerToken always holds a valid f64 string")
    }
}

impl fmt::Display for PricePerToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for PricePerToken {
    type Error = InvalidPricePerToken;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]
    #![expect(
        clippy::float_cmp,
        reason = "transparent string-parse; no arithmetic; exact bit comparison is correct"
    )]

    use super::*;

    // --- constructor acceptance ---

    #[test]
    fn accepts_zero() {
        let p = PricePerToken::new("0").unwrap();
        assert_eq!(p.as_str(), "0");
        assert_eq!(p.to_f64(), 0.0_f64);
    }

    #[test]
    fn accepts_small_decimal() {
        let p = PricePerToken::new("0.0000003").unwrap();
        assert_eq!(p.as_str(), "0.0000003");
    }

    #[test]
    fn accepts_integer_string() {
        let p = PricePerToken::new("1").unwrap();
        assert_eq!(p.to_f64(), 1.0_f64);
    }

    // --- constructor rejection ---

    #[test]
    fn rejects_empty() {
        assert_eq!(
            PricePerToken::new("").unwrap_err(),
            InvalidPricePerToken::Empty
        );
    }

    #[test]
    fn rejects_non_numeric() {
        assert!(matches!(
            PricePerToken::new("not-a-number").unwrap_err(),
            InvalidPricePerToken::NotANumber(_)
        ));
        assert!(matches!(
            PricePerToken::new("abc").unwrap_err(),
            InvalidPricePerToken::NotANumber(_)
        ));
    }

    #[test]
    fn rejects_negative() {
        assert_eq!(
            PricePerToken::new("-1").unwrap_err(),
            InvalidPricePerToken::Negative
        );
        assert_eq!(
            PricePerToken::new("-0.001").unwrap_err(),
            InvalidPricePerToken::Negative
        );
    }

    #[test]
    fn rejects_nan() {
        assert!(matches!(
            PricePerToken::new("NaN").unwrap_err(),
            InvalidPricePerToken::NonFinite
        ));
    }

    #[test]
    fn rejects_infinity_strings() {
        assert!(matches!(
            PricePerToken::new("inf").unwrap_err(),
            InvalidPricePerToken::NonFinite
        ));
        assert!(matches!(
            PricePerToken::new("-inf").unwrap_err(),
            InvalidPricePerToken::NonFinite
        ));
    }

    // --- accessors ---

    #[test]
    fn as_str_returns_original_string() {
        let raw = "0.0000003";
        let p = PricePerToken::new(raw).unwrap();
        assert_eq!(p.as_str(), raw);
    }

    #[test]
    fn to_f64_converts_correctly() {
        let p = PricePerToken::new("2.5").unwrap();
        assert_eq!(p.to_f64(), 2.5_f64);
    }

    // --- Display ---

    #[test]
    fn display_matches_as_str() {
        let p = PricePerToken::new("0.001").unwrap();
        assert_eq!(format!("{p}"), p.as_str());
    }

    // --- serde try_from round-trip ---

    #[test]
    fn deserialize_valid_string_succeeds() {
        let j = r#""0.0000003""#;
        let p: PricePerToken = serde_json::from_str(j).unwrap();
        assert_eq!(p.as_str(), "0.0000003");
    }

    #[test]
    fn deserialize_negative_string_fails() {
        let j = r#""-1""#;
        assert!(serde_json::from_str::<PricePerToken>(j).is_err());
    }

    #[test]
    fn deserialize_non_numeric_string_fails() {
        let j = r#""bad""#;
        assert!(serde_json::from_str::<PricePerToken>(j).is_err());
    }

    #[test]
    fn deserialize_empty_string_fails() {
        let j = r#""""#;
        assert!(serde_json::from_str::<PricePerToken>(j).is_err());
    }

    // --- equality ---

    #[test]
    fn same_string_is_equal() {
        let a = PricePerToken::new("0.5").unwrap();
        let b = PricePerToken::new("0.5").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_representations_are_not_equal() {
        // String-backed: "0.5" != "0.50" even if numerically equal.
        let a = PricePerToken::new("0.5").unwrap();
        let b = PricePerToken::new("0.50").unwrap();
        assert_ne!(a, b);
    }
}
