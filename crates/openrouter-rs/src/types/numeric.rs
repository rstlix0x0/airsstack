//! Bounded numeric newtypes for chat-completion sampling parameters.
//!
//! Each type validates its range at construction so request-building code
//! trusts the bound without re-checking. Ranges follow the OpenRouter
//! OpenAI-compatible chat-completions contract.
//!
//! Responsibilities:
//! - Declare the sampling-parameter newtypes (`MaxTokens`, `Temperature`,
//!   `TopP`, `TopK`, `Seed`, `FrequencyPenalty`, `PresencePenalty`,
//!   `RepetitionPenalty`) and their construction-failure reasons.

/// Returns `true` when `n` is finite and within the inclusive range `lo..=hi`.
fn in_range(n: f32, lo: f32, hi: f32) -> bool {
    n.is_finite() && (lo..=hi).contains(&n)
}

/// Maximum number of tokens to generate. Any non-zero `u32`.
///
/// The SDK imposes no upper bound — per-model output caps shift with each
/// release and the server is authoritative.
///
/// # Examples
/// ```
/// use openrouter_rs::types::MaxTokens;
/// assert_eq!(MaxTokens::new(1024).expect("non-zero").get(), 1024);
/// assert!(MaxTokens::new(0).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct MaxTokens(u32);

/// Reason [`MaxTokens::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("max_tokens must be >= 1 (got 0)")]
pub struct InvalidMaxTokens;

impl MaxTokens {
    /// Validate and wrap a `u32`.
    ///
    /// # Errors
    /// Returns [`InvalidMaxTokens`] when `n` is `0`.
    pub const fn new(n: u32) -> Result<Self, InvalidMaxTokens> {
        if n == 0 {
            return Err(InvalidMaxTokens);
        }
        Ok(Self(n))
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Sampling temperature. Range `0.0..=2.0` (OpenAI-compatible), must be finite.
///
/// # Examples
/// ```
/// use openrouter_rs::types::Temperature;
/// assert!(Temperature::new(0.7).is_ok());
/// assert!(Temperature::new(2.5).is_err());
/// assert!(Temperature::new(f32::NAN).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Temperature(f32);

/// Reason [`Temperature::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("temperature must be finite and within 0.0..=2.0")]
pub struct InvalidTemperature;

impl Temperature {
    /// Validate and wrap an `f32`.
    ///
    /// # Errors
    /// Returns [`InvalidTemperature`] when `n` is non-finite or outside `0.0..=2.0`.
    pub fn new(n: f32) -> Result<Self, InvalidTemperature> {
        if in_range(n, 0.0, 2.0) {
            Ok(Self(n))
        } else {
            Err(InvalidTemperature)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Nucleus-sampling cutoff. Range `0.0..=1.0`, must be finite.
///
/// # Examples
/// ```
/// use openrouter_rs::types::TopP;
/// assert!(TopP::new(0.9).is_ok());
/// assert!(TopP::new(1.5).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TopP(f32);

/// Reason [`TopP::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("top_p must be finite and within 0.0..=1.0")]
pub struct InvalidTopP;

impl TopP {
    /// Validate and wrap an `f32`.
    ///
    /// # Errors
    /// Returns [`InvalidTopP`] when `n` is non-finite or outside `0.0..=1.0`.
    pub fn new(n: f32) -> Result<Self, InvalidTopP> {
        if in_range(n, 0.0, 1.0) {
            Ok(Self(n))
        } else {
            Err(InvalidTopP)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Top-K sampling cutoff. Any `u32`; the server validates per-model support.
///
/// # Examples
/// ```
/// use openrouter_rs::types::TopK;
/// assert_eq!(TopK::new(40).get(), 40);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TopK(u32);

impl TopK {
    /// Wrap a `u32`.
    #[must_use]
    pub const fn new(n: u32) -> Self {
        Self(n)
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Deterministic-sampling seed. Any `u64`.
///
/// # Examples
/// ```
/// use openrouter_rs::types::Seed;
/// assert_eq!(Seed::new(12345).get(), 12345);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Seed(u64);

impl Seed {
    /// Wrap a `u64`.
    #[must_use]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Frequency penalty. Range `-2.0..=2.0`, must be finite.
///
/// # Examples
/// ```
/// use openrouter_rs::types::FrequencyPenalty;
/// assert!(FrequencyPenalty::new(0.5).is_ok());
/// assert!(FrequencyPenalty::new(3.0).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FrequencyPenalty(f32);

/// Reason [`FrequencyPenalty::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("frequency_penalty must be finite and within -2.0..=2.0")]
pub struct InvalidFrequencyPenalty;

impl FrequencyPenalty {
    /// Validate and wrap an `f32`.
    ///
    /// # Errors
    /// Returns [`InvalidFrequencyPenalty`] when `n` is non-finite or outside `-2.0..=2.0`.
    pub fn new(n: f32) -> Result<Self, InvalidFrequencyPenalty> {
        if in_range(n, -2.0, 2.0) {
            Ok(Self(n))
        } else {
            Err(InvalidFrequencyPenalty)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Presence penalty. Range `-2.0..=2.0`, must be finite.
///
/// # Examples
/// ```
/// use openrouter_rs::types::PresencePenalty;
/// assert!(PresencePenalty::new(-1.0).is_ok());
/// assert!(PresencePenalty::new(-2.5).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct PresencePenalty(f32);

/// Reason [`PresencePenalty::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("presence_penalty must be finite and within -2.0..=2.0")]
pub struct InvalidPresencePenalty;

impl PresencePenalty {
    /// Validate and wrap an `f32`.
    ///
    /// # Errors
    /// Returns [`InvalidPresencePenalty`] when `n` is non-finite or outside `-2.0..=2.0`.
    pub fn new(n: f32) -> Result<Self, InvalidPresencePenalty> {
        if in_range(n, -2.0, 2.0) {
            Ok(Self(n))
        } else {
            Err(InvalidPresencePenalty)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Repetition penalty. Range `0.0..=2.0` (default 1.0), must be finite.
///
/// # Examples
/// ```
/// use openrouter_rs::types::RepetitionPenalty;
/// assert!(RepetitionPenalty::new(1.1).is_ok());
/// assert!(RepetitionPenalty::new(2.5).is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct RepetitionPenalty(f32);

/// Reason [`RepetitionPenalty::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq, Eq)]
#[error("repetition_penalty must be finite and within 0.0..=2.0")]
pub struct InvalidRepetitionPenalty;

impl RepetitionPenalty {
    /// Validate and wrap an `f32`.
    ///
    /// # Errors
    /// Returns [`InvalidRepetitionPenalty`] when `n` is non-finite or outside `0.0..=2.0`.
    pub fn new(n: f32) -> Result<Self, InvalidRepetitionPenalty> {
        if in_range(n, 0.0, 2.0) {
            Ok(Self(n))
        } else {
            Err(InvalidRepetitionPenalty)
        }
    }

    /// The inner value, for wire-format use.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "tests unwrap known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::*;

    #[test]
    fn max_tokens_rejects_zero_accepts_positive() {
        assert_eq!(MaxTokens::new(0).unwrap_err(), InvalidMaxTokens);
        assert_eq!(MaxTokens::new(1024).unwrap().get(), 1024);
        assert!(MaxTokens::new(200_000).is_ok());
    }

    #[test]
    fn temperature_bounds() {
        assert!(Temperature::new(0.0).is_ok());
        assert!(Temperature::new(2.0).is_ok());
        assert!(Temperature::new(-0.1).is_err());
        assert!(Temperature::new(2.1).is_err());
        assert!(Temperature::new(f32::NAN).is_err());
        assert!(Temperature::new(f32::INFINITY).is_err());
    }

    #[test]
    fn top_p_bounds() {
        assert!(TopP::new(0.0).is_ok());
        assert!(TopP::new(1.0).is_ok());
        assert!(TopP::new(1.1).is_err());
        assert!(TopP::new(f32::NAN).is_err());
    }

    #[test]
    fn penalties_bounds() {
        assert!(FrequencyPenalty::new(-2.0).is_ok());
        assert!(FrequencyPenalty::new(2.0).is_ok());
        assert!(FrequencyPenalty::new(-2.1).is_err());
        assert!(PresencePenalty::new(2.0).is_ok());
        assert!(PresencePenalty::new(2.1).is_err());
        assert!(PresencePenalty::new(f32::NAN).is_err());
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "transparent newtype stores and returns the exact same bits; no arithmetic"
    )]
    fn repetition_penalty_within_zero_to_two_finite() {
        assert!(RepetitionPenalty::new(0.0).is_ok());
        assert!(RepetitionPenalty::new(2.0).is_ok());
        assert!(RepetitionPenalty::new(2.1).is_err());
        assert!(RepetitionPenalty::new(-1.0).is_err());
        assert!(RepetitionPenalty::new(f32::INFINITY).is_err());
        assert_eq!(RepetitionPenalty::new(1.1).unwrap().get(), 1.1_f32);
    }

    #[test]
    fn top_k_and_seed_are_infallible_wrappers() {
        assert_eq!(TopK::new(40).get(), 40);
        assert_eq!(Seed::new(12345).get(), 12345);
    }

    #[test]
    fn round_trips_serde_transparent() {
        let mt = MaxTokens::new(1024).unwrap();
        assert_eq!(serde_json::to_string(&mt).unwrap(), "1024");
        let back: MaxTokens = serde_json::from_str("1024").unwrap();
        assert_eq!(back, mt);

        let t = Temperature::new(0.7).unwrap();
        let j = serde_json::to_string(&t).unwrap();
        let back_t: Temperature = serde_json::from_str(&j).unwrap();
        assert_eq!(back_t, t);
    }
}
