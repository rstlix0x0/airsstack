//! Runtime-agnostic evals harness for the Agent SDK.
//!
//! Define single-turn eval cases, run them against any [`crate::agent::Client`],
//! score the collected output, and read an aggregate report. Because the harness
//! runs against the `Runtime`/`Client` seam it behaves identically whichever
//! runtime is underneath.
//!
//! ```
//! use clauders::agent::evals::{Case, EvalSuite, contains, no_error};
//!
//! let suite = EvalSuite::new()
//!     .case(Case::new("greets", "hi").scorer(contains("hello")))
//!     .case(Case::new("clean", "run").scorer(no_error()));
//! assert_eq!(suite.len(), 2);
//! ```

pub mod case;
pub mod error;
pub mod judge;
pub mod outcome;
pub mod report;
pub mod runner;
pub mod score;
pub mod scorers;

pub use case::Case;
pub use error::EvalError;
pub use judge::{Grader, Judge};
pub use outcome::Outcome;
pub use report::{CaseReport, Report};
pub use runner::EvalSuite;
pub use score::{Score, Scorer};
pub use scorers::{
    Contains, Equals, NoError, Predicate, TokenBudget, ToolCalled, contains, equals, no_error,
    predicate, token_budget, tool_called,
};
