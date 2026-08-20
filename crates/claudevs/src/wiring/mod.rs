//! Static wiring checks: no execution, one `Finding` list (spec §7).

mod finding;
pub mod invocations;
pub mod refs;

pub use finding::{Finding, Severity, WiringReport};
