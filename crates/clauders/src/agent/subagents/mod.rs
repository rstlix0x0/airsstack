//! Programmatic subagents.
//!
//! Holds the canonical [`AgentDefinition`] value type — one named helper
//! agent the running agent can delegate a subtask to. Carried by
//! [`crate::agent::Options`] and consumed by the runtimes.

mod definition;

pub use definition::{AgentDefinition, AgentDefinitionError};
