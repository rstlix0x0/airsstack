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
#[derive(Clone, Debug)]
pub struct ExpBackoff {
    /// Total number of attempts including the original request.
    pub max_attempts: NonZeroU32,
    /// Backoff applied before the second attempt (attempt index 1).
    pub initial: Duration,
    /// Hard cap on the per-attempt backoff after exponential growth.
    pub max: Duration,
    /// Factor applied at each attempt: `initial * multiplier.pow(attempt)`.
    pub multiplier: f32,
    /// Jitter mode applied on top of the exponential value.
    pub jitter: Jitter,
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
    /// Compute the backoff for the given attempt index (0-based).
    ///
    /// `attempt == 0` returns the configured initial delay; subsequent
    /// attempts multiply by [`ExpBackoff::multiplier`] and cap at
    /// [`ExpBackoff::max`]. `Disabled` always returns [`Duration::ZERO`].
    ///
    /// Jitter is not applied here — the deterministic curve makes the
    /// branch trivial to unit-test. The request-layer caller mixes in a
    /// random sample according to [`ExpBackoff::jitter`].
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
            if secs >= max_secs {
                secs = max_secs;
                break;
            }
        }
        Duration::from_secs_f64(secs.min(max_secs))
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
    #![allow(clippy::unwrap_used, clippy::expect_used)]

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
}
