//! Bounded numeric newtypes used in `MessageRequest` sampling parameters.

/// Maximum number of tokens to generate in the response.
///
/// Valid range: any non-zero `u32`. The SDK does not impose an upper bound
/// here because per-model output caps shift with each Anthropic release
/// (e.g. Sonnet 4.5 accepts up to 64 000, Haiku 4.5 up to 8 192). The
/// server is the authoritative source for the model-specific limit;
/// requesting more than a given model supports returns an `invalid_request_error`.
///
/// # Examples
///
/// ```
/// use clauders::types::MaxTokens;
/// assert_eq!(MaxTokens::new(1024).expect("non-zero").get(), 1024);
/// assert!(MaxTokens::new(0).is_err());
/// // Large values are accepted at the SDK layer — the server validates
/// // the per-model cap.
/// assert!(MaxTokens::new(200_000).is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct MaxTokens(u32);

/// Reason [`MaxTokens::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("max_tokens must be >= 1 (got 0)")]
pub struct InvalidMaxTokens;

impl MaxTokens {
    /// Validate and wrap a `u32` as `MaxTokens`.
    ///
    /// # Errors
    /// Returns [`InvalidMaxTokens`] when `n` is `0`.
    pub const fn new(n: u32) -> Result<Self, InvalidMaxTokens> {
        if n == 0 {
            return Err(InvalidMaxTokens);
        }
        Ok(Self(n))
    }

    /// Borrow the inner value for wire-format use.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Sampling temperature. Valid range: `0.0..=1.0` per the Anthropic API.
///
/// # Examples
///
/// ```
/// use clauders::types::Temperature;
/// assert!(Temperature::new(0.7).is_ok());
/// assert!(Temperature::new(1.5).is_err());
/// assert!(Temperature::new(f32::NAN).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Temperature(f32);

/// Reason [`Temperature::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq)]
#[error("temperature must be in 0.0..=1.0 (got {value})")]
#[non_exhaustive]
pub struct InvalidTemperature {
    /// The rejected value.
    pub value: f32,
}

impl Temperature {
    /// Validate and wrap an `f32` as `Temperature`.
    ///
    /// # Errors
    /// Returns [`InvalidTemperature`] when `v` is outside `0.0..=1.0` or is NaN.
    pub const fn new(v: f32) -> Result<Self, InvalidTemperature> {
        if v.is_nan() || v < 0.0 || v > 1.0 {
            return Err(InvalidTemperature { value: v });
        }
        Ok(Self(v))
    }

    /// Return the inner value for wire-format use.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Top-p nucleus sampling. Valid range: `0.0..=1.0` per the Anthropic API.
///
/// # Examples
///
/// ```
/// use clauders::types::TopP;
/// assert!(TopP::new(0.9).is_ok());
/// assert!(TopP::new(1.5).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TopP(f32);

/// Reason [`TopP::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq)]
#[error("top_p must be in 0.0..=1.0 (got {value})")]
#[non_exhaustive]
pub struct InvalidTopP {
    /// The rejected value.
    pub value: f32,
}

impl TopP {
    /// Validate and wrap an `f32` as `TopP`.
    ///
    /// # Errors
    /// Returns [`InvalidTopP`] when `v` is outside `0.0..=1.0` or is NaN.
    pub const fn new(v: f32) -> Result<Self, InvalidTopP> {
        if v.is_nan() || v < 0.0 || v > 1.0 {
            return Err(InvalidTopP { value: v });
        }
        Ok(Self(v))
    }

    /// Return the inner value for wire-format use.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Top-k sampling. Valid range: `>= 1` per the Anthropic API.
///
/// # Examples
///
/// ```
/// use clauders::types::TopK;
/// assert!(TopK::new(40).is_ok());
/// assert!(TopK::new(0).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TopK(u32);

/// Reason [`TopK::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("top_k must be >= 1 (got 0)")]
pub struct InvalidTopK;

impl TopK {
    /// Validate and wrap a `u32` as `TopK`.
    ///
    /// # Errors
    /// Returns [`InvalidTopK`] when `v` is `0`.
    pub const fn new(v: u32) -> Result<Self, InvalidTopK> {
        if v == 0 {
            return Err(InvalidTopK);
        }
        Ok(Self(v))
    }

    /// Borrow the inner value for wire-format use.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn max_tokens_rejects_zero_accepts_any_positive() {
        assert!(MaxTokens::new(0).is_err());
        assert!(MaxTokens::new(1).is_ok());
        assert!(MaxTokens::new(32_000).is_ok());
        assert!(MaxTokens::new(64_000).is_ok());
        assert!(MaxTokens::new(200_000).is_ok());
        assert_eq!(MaxTokens::new(1024).unwrap().get(), 1024);
    }

    #[test]
    fn temperature_bounds() {
        assert!(Temperature::new(0.0).is_ok());
        assert!(Temperature::new(0.5).is_ok());
        assert!(Temperature::new(1.0).is_ok());
        assert!(Temperature::new(-0.1).is_err());
        assert!(Temperature::new(1.1).is_err());
        assert!(Temperature::new(f32::NAN).is_err());
    }

    #[test]
    fn top_p_bounds() {
        assert!(TopP::new(0.0).is_ok());
        assert!(TopP::new(0.5).is_ok());
        assert!(TopP::new(1.0).is_ok());
        assert!(TopP::new(-0.1).is_err());
        assert!(TopP::new(2.0).is_err());
        assert!(TopP::new(f32::NAN).is_err());
    }

    #[test]
    fn top_k_bounds() {
        assert!(TopK::new(0).is_err());
        assert!(TopK::new(1).is_ok());
        assert!(TopK::new(40).is_ok());
    }

    #[test]
    fn serde_transparent_round_trips() {
        let mt = MaxTokens::new(1024).unwrap();
        let j = serde_json::to_string(&mt).unwrap();
        assert_eq!(j, "1024");
        let back: MaxTokens = serde_json::from_str(&j).unwrap();
        assert_eq!(back, mt);
    }
}
