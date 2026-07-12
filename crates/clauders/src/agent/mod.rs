//! Claude Agent SDK surface for `clauders`.
//!
//! This module tree drives the `claude` Code CLI binary as a subprocess
//! over the control protocol.
//!
//! The data layer covers the error hierarchy, strong types, message and
//! content frames, the capability manifest, `Options`, and the control-protocol
//! codec. The async `Runtime`/`Client` layer is a separate module.

pub mod capabilities;
pub mod client;
pub mod content;
pub mod error;
pub mod evals;
pub mod hooks;
pub mod mcp;
pub mod message;
pub mod middleware;
pub mod options;
pub mod orchestration;
pub mod permissions;
pub mod process;
pub mod protocol;
pub mod runtime;
pub mod stream;
pub mod subagents;
pub mod system_prompt;
pub mod types;

pub use capabilities::{Capabilities, HookEvent};
pub use client::{AgentClientBuilder, Client, query};
pub use content::ContentBlock;
pub use error::AgentError;
pub use evals::{
    Case, CaseReport, EvalError as EvalsError, EvalSuite, Grader, Judge, Outcome, Report, Score,
    Scorer,
};
pub use hooks::{Hook, HookDecision, HookInput, HookOutput, HookRegistry};
pub use mcp::{
    FnTool, SdkMcpRegistry, SdkMcpServer, SdkMcpServerBuilder, Tool, ToolAnnotations, ToolContent,
    ToolResult, tool,
};
pub use message::{
    AssistantMessage, Message, ResultMessage, StreamEvent, SystemMessage, Usage, UserMessage,
};
pub use middleware::{
    Layer, MeterHandle, MeterRuntime, Retry, RetryRuntime, Stack, TokenMeter, Trace, TraceRuntime,
    UsageTotals,
};
pub use options::{Options, OptionsBuilder};
pub use orchestration::{Limiter, Pool, SemaphoreLimiter};
pub use permissions::{
    PermissionBehavior, PermissionContext, PermissionDecision, PermissionMode, PermissionPolicy,
    PermissionScope, PermissionUpdate,
};
pub use runtime::Runtime;
pub use runtime::api::{ApiRuntime, CachePolicy};
pub use runtime::cli::CliRuntime;
pub use runtime::openrouter::OpenRouterRuntime;
pub use runtime::permission_judge::RuntimeJudge;
pub use runtime::routing::{
    Classifier, ModelCard, RoutingError, RoutingRuntime, RoutingRuntimeBuilder, RoutingSummary,
    RuntimeClassifier,
};
pub use stream::MessageStream;
pub use subagents::{AgentDefinition, AgentDefinitionError};
pub use system_prompt::SystemPromptConfig;
pub use types::SessionControl;
