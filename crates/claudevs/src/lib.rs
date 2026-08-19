//! Engine for `claudevs`, the Claude Code plugin lifecycle CLI.
//!
//! Deterministic plugin testing: a canonical case model fed by YAML and Lua
//! front-ends, a harness that spawns a plugin's hooks and scripts the way the
//! Claude Code runtime would, and a report over the verdicts. The `claudevs-cli`
//! crate is the thin binary over this library.

mod error;
mod native;
mod report;
mod suite;

pub mod case;
pub mod harness;
pub mod types;

pub use error::{Error, Result};
pub use native::NativeOutcome;
pub use report::{exit_code, render_human, render_json};
pub use suite::{CaseOutcome, SuiteOptions, SuiteReport, run_case, run_suite};
