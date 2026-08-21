//! Static wiring checks: no execution, one [`Finding`] list.
//!
//! Three checkers, each answering one question about a plugin that never
//! requires running it:
//!
//! - [`refs`] — does every `${CLAUDE_PLUGIN_ROOT}/…` reference resolve, and
//!   does any of them leave the plugin root?
//! - [`invocations`] — is any script in the plugin named by nothing else in it?
//! - [`matchers`] — does hooks.json declare known events and compiling regexes?
//!
//! Responsibilities: the re-exported [`Finding`], [`Severity`],
//! [`WiringReport`], [`FencedCommand`], [`parse_fenced`] and [`run`], which
//! composes the three checkers — see `run` for why it has its own file.

mod finding;
pub mod invocations;
pub mod matchers;
pub mod refs;
mod run;

pub use finding::{Finding, Severity, WiringReport};
pub use invocations::{FencedCommand, parse_fenced};
pub use run::run;
