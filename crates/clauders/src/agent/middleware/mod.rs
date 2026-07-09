//! Middleware backbone: compose runtime-decorating layers over a base runtime.
//!
//! A layer wraps a [`crate::agent::runtime::Runtime`] and is itself a `Runtime`,
//! so a composed stack drops into the generic `Client`. The `Layer` trait and
//! `Stack` builder compose layers at the type level; the shipped layers observe
//! or retry runtime operations without altering the public surface.
//!
//! ```
//! use clauders::agent::{Retry, Stack, TokenMeter, Trace};
//!
//! fn compose<R: clauders::agent::Runtime>(base: R) -> impl clauders::agent::Runtime {
//!     let (meter, _usage) = TokenMeter::new();
//!     Stack::new(base)
//!         .layer(Retry::new(3)) // innermost: retries the transport
//!         .layer(meter)         // meters real usage
//!         .layer(Trace::new())  // outermost: observes the final behavior
//!         .build()
//! }
//! ```

pub mod layer;
pub mod meter;
pub mod retry;
pub(crate) mod tap;
pub mod trace;

pub use layer::{Layer, Stack};
pub use meter::{MeterHandle, MeterRuntime, TokenMeter, UsageTotals};
pub use retry::{Retry, RetryRuntime};
pub use trace::{Trace, TraceRuntime};
