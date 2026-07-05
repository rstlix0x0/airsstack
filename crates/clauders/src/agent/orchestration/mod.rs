//! Bounded-concurrency orchestration for the Agent SDK.
//!
//! A pure, agent-agnostic engine ([`core::engine`]) drives many jobs at once under
//! an admission [`core::limiter::Limiter`], emitting each result as its job finishes.
//! The typed [`Pool`] facade builds on that core: it runs a batch of single-turn
//! prompts concurrently, each on its own runtime, bounded by a [`Limiter`].
//!
//! ```
//! use clauders::agent::orchestration::{Pool, SemaphoreLimiter};
//! use clauders::agent::{CliRuntime, Options};
//!
//! // At most four prompts run concurrently; each builds its own subprocess session.
//! let limiter = SemaphoreLimiter::new(4);
//! assert_eq!(limiter.permits(), 4);
//!
//! let pool = Pool::<CliRuntime, _>::new(
//!     || async { CliRuntime::connect(Options::default()).await },
//!     limiter,
//! );
//! // `pool.run_collect(prompts).await` drives them and returns results in order.
//! let _ = pool;
//! ```

pub mod collect;
pub mod core;
pub mod limit;
pub mod pool;

pub use core::limiter::Limiter;
pub use limit::semaphore::SemaphoreLimiter;
pub use pool::Pool;
