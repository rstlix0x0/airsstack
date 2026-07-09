//! AI-driven per-request model routing.
//!
//! `RoutingRuntime` implements `Runtime` by classifying each prompt (via a
//! `Classifier`) and delegating to the chosen backend runtime. `card` holds the
//! catalog types, `classifier` the decision seam, `builder` the constructor.

mod builder;
mod card;
mod classifier;
mod error;
mod runtime;

pub use builder::{NeedsFallback, Ready, RoutingRuntimeBuilder};
pub use card::{ModelCard, RoutingSummary};
pub use classifier::{Classifier, RuntimeClassifier};
pub use error::RoutingError;
pub use runtime::RoutingRuntime;
