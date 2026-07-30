//! Session-file operations: enumerate, read, rename, and tag stored sessions.
//!
//! These operations are plain local file I/O over the on-disk transcript
//! store; they neither spawn nor drive the `claude` subprocess.
//!
//! # Example
//!
//! ```no_run
//! # async fn run() -> Result<(), clauders::agent::SessionError> {
//! use clauders::agent::{ListOptions, SessionArchive};
//! let archive = SessionArchive::new()?;
//! for session in archive.list(ListOptions::default()).await? {
//!     println!("{}: {}", session.session_id, session.summary);
//! }
//! # Ok(()) }
//! ```

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
