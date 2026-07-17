//! In-process MCP tools for the Agent SDK.
//!
//! Defines the [`Tool`] seam and the [`tool()`] adapter, the `SdkMcpServer`
//! that groups tools, the registry that holds servers for a session, and the
//! JSON-RPC router that answers the methods the backend drives.

pub mod registry;
pub(crate) mod router;
pub mod server;
pub mod tool;

pub use registry::SdkMcpRegistry;
pub use server::{SdkMcpServer, SdkMcpServerBuilder};
pub use tool::{FnTool, ResourceContents, Tool, ToolAnnotations, ToolContent, ToolResult, tool};
