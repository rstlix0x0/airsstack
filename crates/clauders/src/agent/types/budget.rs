//! Client-side spend ceiling for an agent session.

/// A positive US-dollar budget ceiling for a session's API spend.
///
/// Lowers to the binary's `--max-budget-usd <amount>` flag. The value must be
/// finite and strictly greater than zero; the SDK does not impose an upper
/// bound (the binary enforces the ceiling and ends the run with an error
/// result when the client-side cost estimate reaches it).
///
/// # Examples
///
/// ```
/// use clauders::agent::types::BudgetUsd;
/// assert!(BudgetUsd::new(5.0).is_ok());
/// assert!(BudgetUsd::new(0.0).is_err());
/// assert!(BudgetUsd::new(-1.0).is_err());
/// assert!(BudgetUsd::new(f64::NAN).is_err());
/// assert_eq!(BudgetUsd::new(2.5).expect("positive").get(), 2.5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BudgetUsd(f64);

/// Reason [`BudgetUsd::new`] can reject input.
#[derive(Debug, Clone, Copy, thiserror::Error, PartialEq)]
#[error("max_budget_usd must be a finite value > 0 (got {value})")]
#[non_exhaustive]
pub struct InvalidBudgetUsd {
    /// The rejected value.
    pub value: f64,
}

impl BudgetUsd {
    /// Validate and wrap an `f64` as a positive budget.
    ///
    /// # Errors
    /// Returns [`InvalidBudgetUsd`] when `v` is NaN, infinite, or `<= 0.0`.
    pub const fn new(v: f64) -> Result<Self, InvalidBudgetUsd> {
        if v.is_nan() || v.is_infinite() || v <= 0.0 {
            return Err(InvalidBudgetUsd { value: v });
        }
        Ok(Self(v))
    }

    /// Return the inner value for wire-format use.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]

    use super::BudgetUsd;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "asserting the exact literal round-trips through get() unchanged"
    )]
    fn accepts_positive_finite_rejects_others() {
        assert!(BudgetUsd::new(0.01).is_ok());
        assert!(BudgetUsd::new(1000.0).is_ok());
        assert!(BudgetUsd::new(0.0).is_err());
        assert!(BudgetUsd::new(-0.5).is_err());
        assert!(BudgetUsd::new(f64::NAN).is_err());
        assert!(BudgetUsd::new(f64::INFINITY).is_err());
        assert_eq!(BudgetUsd::new(2.5).expect("positive").get(), 2.5);
    }
}
