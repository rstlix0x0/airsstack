//! Runtime-agnostic evals harness for the Agent SDK.
//!
//! Define single-turn eval cases, run them against any [`crate::agent::Client`],
//! score the collected output, and read an aggregate report. Because the harness
//! runs against the `Runtime`/`Client` seam it behaves identically whichever
//! runtime is underneath.

pub mod error;
pub mod outcome;
pub mod score;

pub use error::EvalError;
pub use outcome::Outcome;
pub use score::{Score, Scorer};
