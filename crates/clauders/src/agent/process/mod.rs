//! Protocol-blind subprocess management.
//!
//! Spawns an arbitrary child process, owns its pipes, and tears it down
//! with a zombie/orphan-safe lifecycle. Nothing here knows about the
//! `claude` binary or the control protocol.

mod error;
mod spawn;

pub use error::ProcessError;
pub use spawn::ProcessConfig;
