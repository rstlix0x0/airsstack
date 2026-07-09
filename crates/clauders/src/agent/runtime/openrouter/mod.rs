//! The native `Runtime` over the OpenRouter chat-completions API.
//!
//! Structural twin of the `api` runtime: `convert` is the pure wire↔agent
//! mapping seam, `tools` bridges the in-process MCP registry to the OpenRouter
//! function-tool surface, and `runtime` owns `OpenRouterRuntime` and the spawned
//! agent loop.

mod convert;
mod runtime;
mod tools;

pub use runtime::OpenRouterRuntime;
