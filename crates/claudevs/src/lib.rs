//! Engine for `claudevs`, the Claude Code plugin lifecycle CLI.
//!
//! Deterministic plugin testing: a canonical case model fed by YAML and Lua
//! front-ends, a harness that spawns a plugin's hooks and scripts the way the
//! Claude Code runtime would, and a report over the verdicts. The `claudevs-cli`
//! crate is the thin binary over this library.
//!
//! Wiring's matcher check compiles each hooks.json `matcher` with the `regex`
//! crate, which has no lookaround and no backreferences; a pattern relying on
//! either is reported as a finding even where the runtime would accept it.

mod error;
mod native;
mod report;
mod suite;

pub mod case;
pub mod check;
pub mod doctor;
pub mod harness;
pub mod layout;
pub mod types;
pub mod validate;
pub mod wiring;

pub use check::{CheckReport, Stage, StageStatus};
pub use doctor::{Diagnosis, Probe, ProbeStatus};
pub use error::{Error, Result};
pub use native::NativeOutcome;
pub use report::{
    Report, check_exit_code, doctor_exit_code, exit_code, render_check_human, render_doctor_human,
    render_human, render_json, render_wiring_human,
};
pub use suite::{CaseOutcome, SuiteOptions, SuiteReport, run_case, run_suite, run_suite_installed};
pub use validate::Validation;
pub use wiring::{Finding, Severity, WiringReport};
