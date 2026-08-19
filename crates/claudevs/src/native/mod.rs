//! Native suites a plugin declares outside the case model (`claudevs.toml`).
//!
//! Responsibilities: [`NativeOutcome`], `run_declared`.

#![expect(
    clippy::redundant_pub_crate,
    reason = "explicit pub(crate) documents the crate-wide visibility intent at each item"
)]

mod declared;

pub use declared::NativeOutcome;
pub(crate) use declared::run_declared;
