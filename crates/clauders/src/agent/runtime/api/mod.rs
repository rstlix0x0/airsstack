//! Native Messages API runtime for the Agent SDK.
//!
//! [`ApiRuntime`] implements the [`crate::agent::runtime::Runtime`] seam by
//! driving the agent loop against `POST /v1/messages` in-process, rather than
//! over the CLI control protocol. It reimplements the send → stream → run
//! tools → loop cycle and emits the same message frames the core consumes.
//!
//! ```no_run
//! # async fn example() -> Result<(), clauders::agent::AgentError> {
//! use clauders::agent::Client as AgentClient;
//! use clauders::agent::{ApiRuntime, CachePolicy, Options};
//! use clauders::Client as WireClient;
//! use clauders::types::{ApiKey, MaxTokens, ModelId};
//!
//! let wire = WireClient::builder()
//!     .expect("transport")
//!     .api_key(ApiKey::new("sk-ant-…").expect("key"))
//!     .build()
//!     .expect("client");
//! let options = Options::builder()
//!     .model(ModelId::claude_sonnet_4_5())
//!     .max_tokens(MaxTokens::new(1024).expect("non-zero"))
//!     .build();
//! let runtime = ApiRuntime::new(wire, options)?.with_cache_policy(CachePolicy::Prefix);
//! let agent = AgentClient::with_runtime(runtime);
//! let _stream = agent.query("Hello").await?;
//! # Ok(())
//! # }
//! ```

mod cache;
mod convert;
mod runtime;
mod session_store;
mod subagent;
mod tools;

pub use cache::CachePolicy;
pub use runtime::ApiRuntime;
