//! The crate's error type.
//!
//! One operation-tagged enum: a failing case is a *verdict*, never an `Error` —
//! only the inability to produce a verdict lands here.
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

    /// A manifest is missing, unreadable, or lacks a field the layout needs.
    #[error("manifest `{path}`: {reason}")]
    Manifest {
        /// The file that was read, or looked for.
        path: String,
        /// What was wrong with it.
        reason: String,
    },

    /// No ancestor of the plugin hosts a marketplace manifest.
    ///
    /// Distinct from [`Error::Manifest`] on purpose: this is a property of
    /// *where* the plugin sits, not a defect in the plugin, so the check
    /// pipeline may skip a stage for it. A malformed manifest is never this.
    #[error("marketplace `{path}`: {reason}")]
    Marketplace {
        /// The file that was looked for.
        path: String,
        /// Why the lookup came up empty.
        reason: String,
    },

    /// The simulated install layout could not be built.
    #[error("installed layout: {reason}")]
    Layout {
        /// What stopped it.
        reason: String,
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

    /// A report could not be serialized to JSON.
    ///
    /// The report types this crate emits are all plain, serializable data, so
    /// this is unreachable in practice; it exists so `render_json` returns the
    /// crate's own [`Result`] rather than leaking `serde_json`'s.
    #[error("render report as json: {source}")]
    Render {
        /// The underlying serialization failure.
        source: serde_json::Error,
    },
}
