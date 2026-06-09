//! Pipe ends of a spawned child process, handed to the caller on spawn.
//!
//! Exists as its own module so the pipe-bundle type (`ProcessIo`) has a home
//! that is separate from the supervisor-backed handle (`ManagedProcess`).
//! The two types have different lifetimes and ownership shapes: the caller owns
//! `ProcessIo` exclusively and can move it wherever it needs the I/O, while
//! `ManagedProcess` is the shutdown/wait control surface.
//!
//! Responsibilities:
//! - Define [`ProcessIo`], the three-field struct bundling a child's stdin,
//!   stdout, and stderr after spawn.
//!
//! Not responsible for:
//! - Reading from or writing to the pipes — callers operate on the fields
//!   directly through [`StdoutLines`], [`StderrBuffer`], and [`ChildStdin`].
//! - Lifecycle management — that lives in `handle` / `supervisor`.

use tokio::process::ChildStdin;

use super::pipes::{StderrBuffer, StdoutLines};

/// The pipe ends of a spawned child, handed to the caller.
///
/// Dropping [`ProcessIo::stdin`] sends EOF to the child, which is the
/// primary graceful-shutdown signal for well-behaved children.
pub struct ProcessIo {
    /// Writable stdin of the child.
    pub stdin: ChildStdin,
    /// Line-oriented stdout reader.
    pub stdout: StdoutLines,
    /// Continuously-drained, bounded stderr capture.
    pub stderr: StderrBuffer,
}
