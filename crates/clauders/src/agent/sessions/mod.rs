//! Session-file operations: enumerate, read, rename, and tag stored sessions.
//!
//! These operations are plain local file I/O over the on-disk transcript
//! store; they neither spawn nor drive the `claude` subprocess.

pub mod archive;
pub mod error;
pub mod info;
pub mod message;
pub mod options;
pub mod path;

pub use archive::SessionArchive;
pub use error::SessionError;
pub use info::SessionInfo;
pub use message::{SessionMessage, SessionPayload};
pub use options::{ListOptions, MessagesOptions};
