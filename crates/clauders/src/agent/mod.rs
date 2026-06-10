//! Claude Agent SDK surface for `clauders`.
//!
//! This module tree drives the `claude` Code CLI binary as a subprocess
//! over the control protocol. It is gated behind the `agent` feature.
//!
//! The data layer covers the error hierarchy, strong types, message and
//! content frames, the capability manifest, `Options`, and the control-protocol
//! codec. The async `Runtime`/`Client` layer is a separate module.

pub mod capabilities;
pub mod content;
pub mod error;
pub mod message;
pub mod options;
pub mod permissions;
pub mod process;
pub mod protocol;
pub mod types;

pub use capabilities::{Capabilities, HookEvent};
pub use content::ContentBlock;
pub use error::AgentError;
pub use message::{
    AssistantMessage, Message, ResultMessage, StreamEvent, SystemMessage, Usage, UserMessage,
};
pub use options::{Options, OptionsBuilder};
pub use permissions::PermissionMode;
