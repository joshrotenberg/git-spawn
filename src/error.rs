//! Error types for git-spawn.
//!
//! All commands return [`Result<T, Error>`]. The [`enum@Error`] type is
//! non-exhaustive in spirit: callers should match the variants they care
//! about and fall through to a generic arm.
//!
//! ```no_run
//! use git_spawn::{Error, GitCommand, Repository};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let repo = Repository::open("/path/to/repo")?;
//! match repo.log().max_count(10).execute().await {
//!     Ok(out) => println!("{}", out.stdout),
//!     Err(Error::GitNotFound) => eprintln!("install git first"),
//!     Err(Error::CommandFailed { stderr, exit_code, .. }) => {
//!         eprintln!("git failed (exit {exit_code}):\n{stderr}")
//!     }
//!     Err(Error::Timeout { timeout_seconds }) => {
//!         eprintln!("git didn't respond within {timeout_seconds}s")
//!     }
//!     Err(e) => eprintln!("unexpected: {e}"),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## When each variant occurs
//!
//! - [`Error::GitNotFound`] — the OS reported *file not found* while spawning.
//!   Install git or check `PATH`.
//! - [`Error::CommandFailed`] — git exited non-zero. Read `stderr` for the
//!   user-facing message; keep `stdout` for anything git wrote to the
//!   fast-path.
//! - [`Error::Timeout`] — the process exceeded the duration passed to
//!   [`with_timeout`](crate::GitCommand::with_timeout).
//! - [`Error::Io`] — OS-level failure unrelated to exit status (e.g. cwd
//!   doesn't exist, pipe error while reading output).
//! - [`Error::InvalidConfig`] — a builder was missing a required field
//!   (for example [`MvCommand`](crate::MvCommand) with no source).
//! - [`Error::NotARepository`] — [`Repository::open`](crate::Repository::open)
//!   was called on a path that has no `.git`.
//! - [`Error::ParseError`] — a parser in [`crate::parse`] could not decode
//!   the captured output.
//! - [`Error::UnsupportedVersion`] — reserved for future version gating; not
//!   currently emitted.
//! - [`Error::Custom`] — catch-all for cases the library cannot classify.

use thiserror::Error;

/// Result type for git-spawn operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for all git-spawn operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Git binary not found in PATH.
    #[error("git binary not found in PATH")]
    GitNotFound,

    /// Git version is below the minimum supported.
    #[error("git version {found} is not supported (minimum: {minimum})")]
    UnsupportedVersion {
        /// Version reported by `git --version`.
        found: String,
        /// Minimum required version.
        minimum: String,
    },

    /// A git command exited with a non-zero status.
    #[error("git command failed: {command}")]
    CommandFailed {
        /// The full command line that was executed.
        command: String,
        /// Exit code returned by the command.
        exit_code: i32,
        /// Captured stdout.
        stdout: String,
        /// Captured stderr.
        stderr: String,
    },

    /// Failed to parse git output into a typed value.
    #[error("failed to parse git output: {message}")]
    ParseError {
        /// Description of the parse failure.
        message: String,
    },

    /// Invalid configuration supplied to a builder.
    #[error("invalid configuration: {message}")]
    InvalidConfig {
        /// Description of the misconfiguration.
        message: String,
    },

    /// Operation targeted a path that is not a git repository.
    #[error("not a git repository: {path}")]
    NotARepository {
        /// Path that was expected to be a repo.
        path: String,
    },

    /// IO error while spawning or reading from a git process.
    #[error("io error: {message}")]
    Io {
        /// Human-readable message.
        message: String,
        /// Underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// Command exceeded its configured timeout.
    #[error("operation timed out after {timeout_seconds} seconds")]
    Timeout {
        /// Configured timeout in seconds.
        timeout_seconds: u64,
    },

    /// Generic error with a custom message.
    #[error("{message}")]
    Custom {
        /// Custom error message.
        message: String,
    },
}

impl Error {
    /// Create a [`Error::CommandFailed`].
    pub fn command_failed(
        command: impl Into<String>,
        exit_code: i32,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
    ) -> Self {
        Self::CommandFailed {
            command: command.into(),
            exit_code,
            stdout: stdout.into(),
            stderr: stderr.into(),
        }
    }

    /// Create a [`Error::ParseError`].
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
        }
    }

    /// Create a [`Error::InvalidConfig`].
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create a [`Error::NotARepository`].
    pub fn not_a_repository(path: impl Into<String>) -> Self {
        Self::NotARepository { path: path.into() }
    }

    /// Create a [`Error::Timeout`].
    #[must_use]
    pub fn timeout(timeout_seconds: u64) -> Self {
        Self::Timeout { timeout_seconds }
    }

    /// Create a [`Error::Custom`].
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom {
            message: message.into(),
        }
    }

    /// A coarse category useful for logging and metrics.
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            Self::GitNotFound | Self::UnsupportedVersion { .. } => "prerequisites",
            Self::CommandFailed { .. } | Self::Timeout { .. } => "command",
            Self::ParseError { .. } => "parsing",
            Self::InvalidConfig { .. } => "config",
            Self::NotARepository { .. } => "repository",
            Self::Io { .. } => "io",
            Self::Custom { .. } => "custom",
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            source: err,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories() {
        assert_eq!(Error::GitNotFound.category(), "prerequisites");
        assert_eq!(
            Error::command_failed("git status", 1, "", "").category(),
            "command"
        );
        assert_eq!(Error::parse_error("x").category(), "parsing");
        assert_eq!(Error::not_a_repository("/tmp").category(), "repository");
    }

    #[test]
    fn from_io_error() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "nope");
        let err: Error = io.into();
        assert!(matches!(err, Error::Io { .. }));
    }
}
