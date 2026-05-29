//! Retry policy and backoff arithmetic for the SDK request path.
//!
//! Lives apart from `Config` because retry behaviour is orthogonal to the
//! static request metadata: a caller may pin a single `Config` and swap the
//! retry strategy per call site (tests vs production), or vice versa. The
//! arithmetic is pure — no I/O, no clock — so unit tests cover every
//! branch deterministically.
//!
//! Responsibilities:
//! - Declare [`RetryPolicy`], the closed set of strategies the request
//!   path supports (`Disabled` or `ExponentialBackoff(ExpBackoff)`).
//! - Declare [`ExpBackoff`] with its tunable parameters and a sensible
//!   default (3 attempts, 250ms..8s, multiplier 2.0, full jitter).
//! - Declare [`Jitter`], the closed set of jitter modes.
//! - Compute the per-attempt backoff via [`RetryPolicy::backoff`] and the
//!   total attempt count via [`RetryPolicy::max_attempts`].
//!
//! Not responsible for:
//! - Honouring `Retry-After` response headers — that lives in the request
//!   layer, which combines server hints with the policy declared here.
//! - Sleeping between attempts — callers receive a `Duration` and decide
//!   how to wait (typically `tokio::time::sleep`).

use std::num::NonZeroU32;
use std::time::Duration;

/// Jitter mode applied on top of the exponential backoff curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Jitter {
    /// No jitter — use the raw exponential value.
    None,
    /// Symmetric jitter — half-fixed, half-random around the curve.
    Equal,
    /// Full jitter — random uniform sample in `[0, curve]`.
    Full,
}

/// Tunable parameters for the exponential-backoff retry strategy.
///
/// Fields are crate-private so the validated [`ExpBackoff::try_new`]
/// constructor is the only public way to build one — a struct literal
/// cannot bypass its `multiplier` and `initial <= max` checks. Read the
/// parameters back through the accessor methods.
#[derive(Clone, Debug)]
pub struct ExpBackoff {
    pub(crate) max_attempts: NonZeroU32,
    pub(crate) initial: Duration,
    pub(crate) max: Duration,
    pub(crate) multiplier: f32,
    pub(crate) jitter: Jitter,
}

impl ExpBackoff {
    /// Total number of attempts including the original request.
    #[must_use]
    pub const fn max_attempts(&self) -> NonZeroU32 {
        self.max_attempts
    }

    /// Backoff applied before the first retry (attempt index 0).
    ///
    /// [`RetryPolicy::backoff(0)`](RetryPolicy::backoff) returns this value
    /// verbatim; subsequent retries multiply by [`ExpBackoff::multiplier`]
    /// and cap at [`ExpBackoff::max`].
    #[must_use]
    pub const fn initial(&self) -> Duration {
        self.initial
    }

    /// Hard cap on the per-attempt backoff after exponential growth.
    #[must_use]
    pub const fn max(&self) -> Duration {
        self.max
    }

    /// Factor applied at each attempt: `initial * multiplier.pow(attempt)`.
    #[must_use]
    pub const fn multiplier(&self) -> f32 {
        self.multiplier
    }

    /// Jitter mode applied on top of the exponential value.
    #[must_use]
    pub const fn jitter(&self) -> Jitter {
        self.jitter
    }
}

/// Reasons [`ExpBackoff::try_new`] can reject input.
#[derive(Debug, Clone, thiserror::Error, PartialEq)]
#[non_exhaustive]
pub enum InvalidExpBackoff {
    /// `multiplier` was NaN or infinite.
    #[error("multiplier must be finite, got {0}")]
    NonFiniteMultiplier(f32),
    /// `multiplier` was not strictly positive.
    #[error("multiplier must be > 0, got {0}")]
    NonPositiveMultiplier(f32),
    /// `initial > max` — the cap would be exceeded by the very first attempt.
    #[error("initial ({initial:?}) must be <= max ({max:?})")]
    InitialExceedsMax {
        /// Configured initial backoff.
        initial: Duration,
        /// Configured maximum backoff.
        max: Duration,
    },
}

impl ExpBackoff {
    /// Construct a validated `ExpBackoff`.
    ///
    /// # Errors
    /// Returns [`InvalidExpBackoff`] when `multiplier` is non-finite or
    /// non-positive, or when `initial > max`.
    pub fn try_new(
        max_attempts: NonZeroU32,
        initial: Duration,
        max: Duration,
        multiplier: f32,
        jitter: Jitter,
    ) -> Result<Self, InvalidExpBackoff> {
        if !multiplier.is_finite() {
            return Err(InvalidExpBackoff::NonFiniteMultiplier(multiplier));
        }
        if multiplier <= 0.0 {
            return Err(InvalidExpBackoff::NonPositiveMultiplier(multiplier));
        }
        if initial > max {
            return Err(InvalidExpBackoff::InitialExceedsMax { initial, max });
        }
        Ok(Self {
            max_attempts,
            initial,
            max,
            multiplier,
            jitter,
        })
    }
}

impl Default for ExpBackoff {
    #[expect(
        clippy::expect_used,
        reason = "literal 3 is a positive integer; NonZeroU32::new always returns Some"
    )]
    fn default() -> Self {
        Self {
            max_attempts: NonZeroU32::new(3).expect("invariant: 3 > 0"),
            initial: Duration::from_millis(250),
            max: Duration::from_secs(8),
            multiplier: 2.0,
            jitter: Jitter::Full,
        }
    }
}

/// Retry behaviour for the SDK request path.
///
/// `Disabled` skips retries entirely; the first failure is final.
/// `ExponentialBackoff(_)` applies the configured curve and honours
/// server-supplied `Retry-After` headers at the request-layer combiner.
#[derive(Clone, Debug)]
pub enum RetryPolicy {
    /// Retries off — the first failure is the final result.
    Disabled,
    /// Apply the supplied exponential-backoff curve.
    ExponentialBackoff(ExpBackoff),
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::ExponentialBackoff(ExpBackoff::default())
    }
}

impl RetryPolicy {
    /// Backoff applied before retry index `attempt` (0-based).
    ///
    /// `attempt = 0` is the delay before the FIRST retry attempt and
    /// returns [`ExpBackoff::initial`] verbatim; each subsequent retry
    /// multiplies by [`ExpBackoff::multiplier`] and caps at
    /// [`ExpBackoff::max`]. [`RetryPolicy::Disabled`] always returns
    /// [`Duration::ZERO`].
    ///
    /// The original (non-retry) request pays no backoff; callers do not
    /// call this for it. Jitter is intentionally not applied here — the
    /// deterministic curve keeps unit tests trivial; the request layer
    /// mixes in a random sample according to [`ExpBackoff::jitter`].
    #[must_use]
    pub fn backoff(&self, attempt: u32) -> Duration {
        let (initial, max, multiplier) = match self {
            Self::Disabled => return Duration::ZERO,
            Self::ExponentialBackoff(c) => (c.initial, c.max, c.multiplier),
        };

        let mut secs = initial.as_secs_f64();
        let max_secs = max.as_secs_f64();
        for _ in 0..attempt {
            secs *= f64::from(multiplier);
            if !secs.is_finite() || secs >= max_secs {
                return max;
            }
        }
        let final_secs = secs.min(max_secs);
        if !final_secs.is_finite() || final_secs < 0.0 {
            return max;
        }
        Duration::from_secs_f64(final_secs)
    }

    /// Total attempt count including the original request.
    ///
    /// `Disabled` returns `1` — the original attempt with no retry.
    #[must_use]
    pub const fn max_attempts(&self) -> u32 {
        match self {
            Self::Disabled => 1,
            Self::ExponentialBackoff(c) => c.max_attempts.get(),
        }
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
    fn disabled_returns_zero_backoff() {
        assert_eq!(RetryPolicy::Disabled.backoff(0), Duration::ZERO);
        assert_eq!(RetryPolicy::Disabled.backoff(5), Duration::ZERO);
        assert_eq!(RetryPolicy::Disabled.max_attempts(), 1);
    }

    #[test]
    fn exp_backoff_grows_then_caps() {
        let p = RetryPolicy::default();
        let b0 = p.backoff(0);
        let b1 = p.backoff(1);
        let b2 = p.backoff(2);
        let b_huge = p.backoff(20);

        assert!(b0 < b1, "{b0:?} < {b1:?}");
        assert!(b1 < b2, "{b1:?} < {b2:?}");
        assert!(
            b_huge <= Duration::from_secs(8),
            "backoff must cap at 8s: {b_huge:?}"
        );
    }

    #[test]
    fn exp_backoff_initial_matches_default() {
        let p = RetryPolicy::default();
        assert_eq!(p.backoff(0), Duration::from_millis(250));
    }

    #[test]
    fn try_new_rejects_non_finite_multiplier() {
        let r = ExpBackoff::try_new(
            NonZeroU32::new(3).unwrap(),
            Duration::from_millis(250),
            Duration::from_secs(8),
            f32::NAN,
            Jitter::Full,
        );
        assert!(matches!(r, Err(InvalidExpBackoff::NonFiniteMultiplier(_))));
    }

    #[test]
    fn try_new_rejects_non_positive_multiplier() {
        let r = ExpBackoff::try_new(
            NonZeroU32::new(3).unwrap(),
            Duration::from_millis(250),
            Duration::from_secs(8),
            0.0,
            Jitter::Full,
        );
        assert!(matches!(
            r,
            Err(InvalidExpBackoff::NonPositiveMultiplier(_))
        ));
    }

    #[test]
    fn try_new_rejects_initial_exceeds_max() {
        let r = ExpBackoff::try_new(
            NonZeroU32::new(3).unwrap(),
            Duration::from_secs(10),
            Duration::from_secs(5),
            2.0,
            Jitter::None,
        );
        assert!(matches!(
            r,
            Err(InvalidExpBackoff::InitialExceedsMax { .. })
        ));
    }

    #[test]
    fn accessors_round_trip_try_new_inputs() {
        let b = ExpBackoff::try_new(
            NonZeroU32::new(5).unwrap(),
            Duration::from_millis(100),
            Duration::from_secs(4),
            1.5,
            Jitter::Equal,
        )
        .unwrap();
        assert_eq!(b.max_attempts().get(), 5);
        assert_eq!(b.initial(), Duration::from_millis(100));
        assert_eq!(b.max(), Duration::from_secs(4));
        assert!((b.multiplier() - 1.5).abs() < f32::EPSILON);
        assert_eq!(b.jitter(), Jitter::Equal);
    }

    #[test]
    fn backoff_caps_on_nan_multiplier_without_panic() {
        // Caller bypassed `try_new` and built ExpBackoff manually with a NaN
        // multiplier; the curve must clamp to `max` rather than panic in
        // `Duration::from_secs_f64`.
        let policy = RetryPolicy::ExponentialBackoff(ExpBackoff {
            max_attempts: NonZeroU32::new(3).unwrap(),
            initial: Duration::from_millis(100),
            max: Duration::from_secs(2),
            multiplier: f32::NAN,
            jitter: Jitter::None,
        });
        let d = policy.backoff(2);
        assert_eq!(d, Duration::from_secs(2));
    }
}
