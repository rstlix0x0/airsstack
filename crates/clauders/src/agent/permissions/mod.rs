//! Permission control for the agent.
//!
//! Defines the [`PermissionMode`] data enum forwarded to the binary on the
//! `set_permission_mode` control request and carried in [`crate::agent::Options`].
//! Also defines [`PermissionContext`], [`PermissionDecision`], and the
//! [`PermissionPolicy`] trait used by the runtime's in-loop permission handler.

mod decision;
mod judge;
mod mode;
mod policy;
mod update;

pub use decision::{PermissionContext, PermissionDecision};
pub use judge::{JudgeRequest, JudgeRubric, PermissionJudge, RubricError};
pub use mode::PermissionMode;
pub use policy::PermissionPolicy;
pub use update::{PermissionBehavior, PermissionScope, PermissionUpdate};
