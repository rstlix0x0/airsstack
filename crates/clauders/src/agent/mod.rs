//! Claude Agent SDK surface for `clauders`.
//!
//! This module tree drives the `claude` Code CLI binary as a subprocess
//! over the control protocol. It is gated behind the `agent` feature.
//!
//! The data layer covers the error hierarchy, strong types, message and
//! content frames, the capability manifest, `Options`, and the control-protocol
//! codec. The async `Runtime`/`Client` layer is a separate module.

pub mod capabilities;
pub mod cli;
pub mod client;
pub mod content;
pub mod error;
pub mod hooks;
pub mod message;
pub mod options;
pub mod permissions;
pub mod process;
pub mod protocol;
pub mod runtime;
pub mod stream;
pub mod types;

pub use capabilities::{Capabilities, HookEvent};
pub use cli::CliRuntime;
pub use client::{AgentClientBuilder, Client, query};
pub use content::ContentBlock;
pub use error::AgentError;
pub use hooks::{Hook, HookDecision, HookInput, HookOutput, HookRegistry};
pub use message::{
    AssistantMessage, Message, ResultMessage, StreamEvent, SystemMessage, Usage, UserMessage,
};
pub use options::{Options, OptionsBuilder};
pub use permissions::{PermissionContext, PermissionDecision, PermissionMode, PermissionPolicy};
pub use runtime::Runtime;
pub use stream::MessageStream;

#[cfg(feature = "__test-mocks")]
pub mod mock;

#[cfg(feature = "__test-mocks")]
pub use mock::{ControlCall, MockRuntime};
