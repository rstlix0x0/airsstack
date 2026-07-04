//! Middleware backbone: compose runtime-decorating layers over a base runtime.
//!
//! A layer wraps a [`crate::agent::runtime::Runtime`] and is itself a `Runtime`,
//! so a composed stack drops into the generic `Client`. The `Layer` trait and
//! `Stack` builder compose layers at the type level; the shipped layers observe
//! or retry runtime operations without altering the public surface.

pub mod layer;
pub mod meter;
pub mod retry;
pub(crate) mod tap;
pub mod trace;

pub use layer::{Layer, Stack};
pub use meter::{MeterHandle, MeterRuntime, TokenMeter, UsageTotals};
pub use retry::{Retry, RetryRuntime};
pub use trace::{Trace, TraceRuntime};
