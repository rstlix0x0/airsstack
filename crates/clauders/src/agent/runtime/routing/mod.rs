//! AI-driven per-request model routing.
//!
//! `RoutingRuntime` implements `Runtime` by classifying each prompt (via a
//! `Classifier`) and delegating to the chosen backend runtime.

mod error;

pub use error::RoutingError;
