//! The runtime layer: the `Runtime` port and its implementations.
//!
//! `port` defines the single trait seam; `cli` drives the `claude` binary as a
//! subprocess; `mock` is the test double. Everything above this layer (`Client`)
//! is generic over the `Runtime` trait re-exported here.

pub mod cli;
mod port;

pub use port::Runtime;

#[cfg(test)]
pub mod mock;
