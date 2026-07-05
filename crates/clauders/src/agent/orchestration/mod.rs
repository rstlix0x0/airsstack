//! Bounded-concurrency orchestration for the Agent SDK.
//!
//! A pure, agent-agnostic engine ([`core`]) drives many jobs at once under an
//! admission [`core::limiter::Limiter`], emitting each result as its job
//! finishes. Concrete adapters — a semaphore limiter, the typed [`pool::Pool`]
//! facade, and an ordered-collect helper — build on that core.

pub mod collect;
pub mod core;
pub mod limit;
