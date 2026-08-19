//! Validating newtypes for claudevs domain values.

mod case_name;
mod hook_event;

pub use case_name::{CaseName, InvalidCaseName};
pub use hook_event::{HookEvent, InvalidHookEvent};
