//! In-process MCP tools for the Agent SDK.
//!
//! Defines the [`Tool`] seam and the [`tool()`] adapter, the `SdkMcpServer`
//! that groups tools, the registry that holds servers for a session, and the
//! JSON-RPC router that answers the methods the backend drives.

pub mod tool;

pub use tool::{FnTool, Tool, ToolAnnotations, ToolContent, ToolResult, tool};
