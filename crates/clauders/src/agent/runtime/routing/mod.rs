//! AI-driven per-request model routing.
//!
//! `RoutingRuntime` implements `Runtime` by classifying each prompt (via a
//! `Classifier`) and delegating to the chosen backend runtime.

mod card;
mod classifier;
mod error;

pub use card::{ModelCard, RoutingSummary};
pub use classifier::{Classifier, RuntimeClassifier};
pub use error::RoutingError;
