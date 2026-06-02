//! Non-negative, finite price newtype for provider routing limits.
//!
//! Exists as its own file to enforce the non-negative and finite constraints
//! on USD price values at construction time, so routing-preference code never
//! re-checks the invariant.
//!
//! Responsibilities:
//! - [`Price`] — a validated, serializable USD-per-million-tokens price.
//! - [`InvalidPrice`] — the error returned when construction fails.
//!
//! Not responsible for the provider-preferences structure — see
//! `chat/provider.rs`.

/// A non-negative, finite USD price used in provider routing limits.
///
/// The wire format accepts numbers for price values (USD per million tokens).
/// This type rejects `NaN`, positive or negative infinity, and negative values
/// at construction.
///
/// # Examples
///
/// ```
/// use openrouter_rs::types::Price;
///
/// let p = Price::new(0.0).expect("zero is valid");
/// assert_eq!(p.get(), 0.0);
///
/// let p2 = Price::new(2.50).expect("positive is valid");
/// assert!(p2.get() > 0.0);
///
/// assert!(Price::new(-1.0).is_err());
/// assert!(Price::new(f64::NAN).is_err());
/// assert!(Price::new(f64::INFINITY).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Price(f64);

/// Reasons [`Price::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum InvalidPrice {
    /// Input was `NaN`, infinite, or negative.
    #[error("price must be a non-negative, finite number")]
    Invalid,
}

impl Price {
    /// Validate and wrap an `f64`.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidPrice::Invalid`] when `n` is `NaN`, infinite, or
    /// negative.
    pub fn new(n: f64) -> Result<Self, InvalidPrice> {
        if n.is_finite() && n >= 0.0 {
            Ok(Self(n))
        } else {
            Err(InvalidPrice::Invalid)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
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
        reason = "transparent newtype stores and returns exact bits; no arithmetic"
    )]

    use super::*;

    #[test]
    fn accepts_zero() {
        let p = Price::new(0.0).unwrap();
        assert_eq!(p.get(), 0.0);
    }

    #[test]
    fn accepts_positive() {
        let p = Price::new(2.5).unwrap();
        assert_eq!(p.get(), 2.5);
    }

    #[test]
    fn rejects_negative() {
        assert_eq!(Price::new(-1.0).unwrap_err(), InvalidPrice::Invalid);
        assert_eq!(Price::new(-0.001).unwrap_err(), InvalidPrice::Invalid);
    }

    #[test]
    fn rejects_nan() {
        assert_eq!(Price::new(f64::NAN).unwrap_err(), InvalidPrice::Invalid);
    }

    #[test]
    fn rejects_positive_infinity() {
        assert_eq!(
            Price::new(f64::INFINITY).unwrap_err(),
            InvalidPrice::Invalid
        );
    }

    #[test]
    fn rejects_negative_infinity() {
        assert_eq!(
            Price::new(f64::NEG_INFINITY).unwrap_err(),
            InvalidPrice::Invalid
        );
    }

    #[test]
    fn serde_transparent_emits_number() {
        let p = Price::new(1.5).unwrap();
        let j = serde_json::to_string(&p).unwrap();
        assert_eq!(j, "1.5");
        let back: Price = serde_json::from_str(&j).unwrap();
        assert_eq!(back, p);
    }
}
