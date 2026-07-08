//! The runtime layer: the `Runtime` port and its implementations.
//!
//! `port` defines the single trait seam; `api` and `openrouter` are the two
//! native HTTP-API adapters; `cli` drives the `claude` binary as a subprocess;
//! `routing` is the meta-adapter that dispatches each turn to one of the others;
//! `mock` is the test double. Everything above this layer (`Client`) is generic
//! over the `Runtime` trait re-exported here.

pub mod api;
pub mod cli;
pub mod openrouter;
mod port;
pub mod routing;

pub use port::Runtime;

#[cfg(test)]
pub mod mock;
