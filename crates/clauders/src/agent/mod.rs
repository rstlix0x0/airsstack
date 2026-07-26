//! Claude Agent SDK surface for `clauders`.
//!
//! This module tree drives the `claude` Code CLI binary as a subprocess
//! over the control protocol.
//!
//! The data layer covers the error hierarchy, strong types, message and
//! content frames, the capability manifest, `Options`, and the control-protocol
//! codec. The async `Runtime`/`Client` layer is a separate module.

pub mod cancel;
pub mod capabilities;
pub mod client;
pub mod content;
pub mod elicitation;
pub mod error;
pub mod hooks;
pub mod mcp;
pub mod message;
mod model_usage;
pub mod options;
pub mod permissions;
pub mod process;
pub mod protocol;
pub mod runtime;
pub mod sessions;
pub mod stream;
pub mod subagents;
pub mod system_prompt;
pub mod types;
mod warm;

pub use cancel::CancelSignal;
pub use capabilities::{Capabilities, HookEvent};
pub use client::{AgentClientBuilder, Client, query};
pub use content::ContentBlock;
pub use elicitation::{
    ElicitationMode, ElicitationPolicy, ElicitationRequest, ElicitationResponse,
};
pub use error::AgentError;
pub use hooks::{Hook, HookDecision, HookInput, HookOutput, HookRegistry};
pub use mcp::{
    FnTool, ResourceContents, SdkMcpRegistry, SdkMcpServer, SdkMcpServerBuilder, Tool,
    ToolAnnotations, ToolContent, ToolResult, tool,
};
pub use message::{
    AssistantMessage, Message, ResultMessage, StreamEvent, SystemMessage, Usage, UserMessage,
};
pub use model_usage::ModelUsage;
pub use options::{Options, OptionsBuilder};
pub use permissions::{
    MatchedAskRule, PermissionBehavior, PermissionContext, PermissionDecision, PermissionMode,
    PermissionPolicy, PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination,
};
pub use runtime::Runtime;
pub use runtime::cli::CliRuntime;
pub use sessions::{
    ListOptions, MessagesOptions, SessionArchive, SessionError, SessionInfo, SessionMessage,
    SessionPayload,
};
pub use stream::MessageStream;
pub use subagents::{AgentDefinition, AgentDefinitionError, MemorySource};
pub use system_prompt::SystemPromptConfig;
pub use types::{EffortLevel, SessionControl};
pub use warm::{WarmQuery, WarmSession};
