//! The subprocess-backed runtime adapter.
//!
//! Locates and version-checks the backend binary, maps session options to its
//! argument vector, runs the initialize handshake, and demultiplexes its
//! output stream into messages and control responses. Protocol-aware but
//! defers all process lifecycle to the protocol-blind `process` module.

mod argv;
mod demux;
mod discovery;
mod dispatch;
mod handshake;
mod runtime;

pub use runtime::CliRuntime;
