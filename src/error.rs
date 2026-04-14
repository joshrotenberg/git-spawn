//! Error types for git-wrapper.
//!
//! All commands return [`Result<T, Error>`]. Match on specific variants for
//! detailed handling.

use thiserror::Error;

/// Result type for git-wrapper operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for all git-wrapper operations.
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
