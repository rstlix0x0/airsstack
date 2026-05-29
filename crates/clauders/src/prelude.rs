//! Common imports for `use clauders::prelude::*;`.
//!
//! This module re-exports the types most callers need for every request.
//! It is a pure re-export module; no logic lives here.
//!
//! Export-only module — no inline tests per the unit-test mandate exemption
//! for pure re-export modules (exemption #1).

pub use crate::types::{
    AnthropicVersion, ApiKey, BetaHeader, MaxTokens, ModelId, Temperature, TopK, TopP,
};
pub use crate::{ApiError, BuildError, Client, Error, TransportError};

#[cfg(feature = "messages")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages")))]
pub use crate::messages::{ContentBlock, Message, MessageRequest, Role, StopReason};

#[cfg(feature = "messages-streaming")]
#[cfg_attr(docsrs, doc(cfg(feature = "messages-streaming")))]
pub use crate::messages::{MessageStream, StreamEvent};
