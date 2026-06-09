//! Claude Agent SDK surface for `clauders`.
//!
//! This module tree drives the `claude` Code CLI binary as a subprocess
//! over the control protocol. It is gated behind the `agent` feature.

pub mod process;
