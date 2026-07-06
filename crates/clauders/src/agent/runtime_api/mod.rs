//! Native Messages API runtime for the Agent SDK.
//!
//! `ApiRuntime` implements the [`crate::agent::runtime::Runtime`] seam by
//! driving the agent loop against `POST /v1/messages` in-process, rather than
//! over the CLI control protocol. It reimplements the send → stream → run
//! tools → loop cycle and emits the same message frames the core consumes.

mod convert;
