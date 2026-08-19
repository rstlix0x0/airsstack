//! The execution harness: spawning hooks and scripts the way the Claude Code
//! runtime would, and judging what they produced.
//!
//! Responsibilities: [`default_payload`], [`merge`], [`substitute_project`],
//! [`Project`], [`overlay_into`], [`base_env`], [`Captured`], [`run`],
//! [`run_shell`], [`Observed`], [`observe`], [`Verdict`], [`judge`],
//! [`commands_for`], [`resolve_hook`], [`TModule`].

mod environment;
mod hooks_file;
mod payload;
mod project;
mod semantics;
mod spawn;
mod t_module;
mod verdict;

pub use environment::base_env;
pub use hooks_file::{commands_for, resolve as resolve_hook};
pub use payload::{default_payload, merge, substitute_project};
pub use project::{Project, overlay_into};
pub use semantics::{Observed, observe};
pub use spawn::{Captured, DEFAULT_TIMEOUT, run, run_shell};
pub use t_module::TModule;
pub use verdict::{Verdict, judge};
