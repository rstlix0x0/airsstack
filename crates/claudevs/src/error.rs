//! The crate's error type.
//!
//! One operation-tagged enum: a failing case is a *verdict*, never an `Error` —
//! only the inability to produce a verdict lands here (spec §10.1).
//!
//! Responsibilities: [`Error`] and [`Result`].

/// Convenience alias for results carrying [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Why claudevs could not produce a verdict.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A case file could not be loaded or parsed (an author mistake).
    #[error("cannot load case `{path}`: {reason}")]
    CaseLoad {
        /// The file that failed to load.
        path: String,
        /// What was wrong with it.
        reason: String,
    },

    /// A named fixture directory is missing or unreadable.
    #[error("fixture `{name}`: {reason}")]
    Fixture {
        /// The fixture the case named.
        name: String,
        /// What went wrong.
        reason: String,
    },

    /// A hook reference matched zero or several hooks.json entries.
    #[error("hook resolution: {reason}")]
    HookResolution {
        /// Why the reference was ambiguous or unmatched.
        reason: String,
    },

    /// Case discovery found nothing to run.
    #[error(
        "no case files found under `{root}` (cases are `*.yaml`, `*_test.lua` or `test_*.lua` in tests/)"
    )]
    NoCases {
        /// The directory that was searched.
        root: String,
    },

    /// An I/O operation failed.
    #[error("{operation} `{path}`: {source}")]
    Io {
        /// What was being attempted.
        operation: &'static str,
        /// The path involved.
        path: String,
        /// The underlying error.
        source: std::io::Error,
    },

    /// The embedded Lua engine failed outside any single case.
    #[error("lua engine: {reason}")]
    Engine {
        /// What the engine reported.
        reason: String,
    },

    /// A declared native suite could not be started.
    #[error("native suite `{command}`: {reason}")]
    Native {
        /// The declared command.
        command: String,
        /// Why it could not run.
        reason: String,
    },
}
