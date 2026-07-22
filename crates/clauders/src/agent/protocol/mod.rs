//! Control-protocol wire types and line codec.
//!
//! Protocol-aware but transport-blind: these types describe the JSON frames
//! that ride over the subprocess pipes, and the codec turns lines into frames
//! and frames into lines. Nothing here spawns or signals a process.

mod codec;
mod frames;

pub use codec::{RequestId, RequestIdGen, decode_inbound, encode_line};
pub use frames::{
    ControlCancelRequest, ControlResponse, ControlResponseBody, InboundControlRequest,
    InboundFrame, InboundRequestBody, KeepAlive, OutboundControlRequest, OutboundControlResponse,
    OutboundRequestBody, OutboundResponseBody,
};
