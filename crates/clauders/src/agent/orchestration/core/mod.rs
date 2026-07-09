//! The pure orchestration core: the admission seam and the driver.
//!
//! Nothing here knows about agents, prompts, or subprocesses. The core is a
//! bounded-concurrency transform over a channel of tagged futures.

pub mod engine;
pub mod limiter;
