//! Protocol-blind subprocess management for arbitrary child processes.
//!
//! Exists as its own module so the full spawn/supervise/teardown lifecycle can
//! be developed, tested, and reasoned about independently of the `claude`
//! binary or the JSONL control protocol that rides on top of it. Nothing in
//! this module knows about the outer agent.
//!
//! Responsibilities:
//! - [`ProcessConfig`] — declarative spawn parameters (program, args, cwd,
//!   env, grace period).
//! - [`ProcessError`] — all failure modes of the subprocess layer.
//! - [`ManagedProcess`] — owned handle for shutdown/wait/id; co-locates its
//!   `Drop` impl so a dropped handle never orphans the child.
//! - [`ProcessIo`] — the three pipe ends (stdin/stdout/stderr) returned to the
//!   caller at spawn time.
//! - [`StdoutLines`] / [`StderrBuffer`] — line-oriented stdout and bounded
//!   stderr drain.
//!
//! Not responsible for:
//! - Interpreting the bytes that flow over the pipes — the layer above parses
//!   the JSONL control protocol.
//! - Retry, reconnect, or session management — those live outside this module.
//!
//! Entry point: [`ManagedProcess::spawn`].

mod error;
mod handle;
mod io;
mod pipes;
mod spawn;
mod supervisor;

pub use error::ProcessError;
pub use handle::ManagedProcess;
pub use io::ProcessIo;
pub use pipes::{StderrBuffer, StdoutLines};
pub use spawn::ProcessConfig;
