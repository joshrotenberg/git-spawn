//! Command execution primitives.
//!
//! Every git subcommand wrapper is a struct that implements [`GitCommand`].
//! The trait gives each command:
//!
//! - [`execute()`](GitCommand::execute) — run and return a typed output
//! - [`arg()`](GitCommand::arg) / [`args()`](GitCommand::args) — append raw
//!   CLI arguments (escape hatch)
//! - [`with_timeout()`](GitCommand::with_timeout) — cap execution time
//! - [`current_dir()`](GitCommand::current_dir) / [`env()`](GitCommand::env) —
//!   control the subprocess environment
//!
//! Under the hood, each command delegates to a shared [`CommandExecutor`] that
//! spawns `git` via [`tokio::process::Command`], captures stdout/stderr, and
//! maps non-zero exits to [`Error::CommandFailed`].
//!
//! # The two-tier output model
//!
//! Commands with unstructured output — porcelain that varies by git version,
//! locale, and config — return [`CommandOutput`]. Callers can treat stdout as
//! bytes or pass it through a parser in [`crate::parse`].
//!
//! Commands whose output is stable enough to decode return typed values
//! directly. Examples:
//!
//! - [`InitCommand`](init::InitCommand) and [`CloneCommand`](clone::CloneCommand)
//!   return [`Repository`](crate::Repository).
//! - [`RevParseCommand`](rev_parse::RevParseCommand) returns a trimmed
//!   [`String`] (typically a SHA or a boolean-ish literal).
//! - [`CatFileCommand`](cat_file::CatFileCommand) returns the object body as
//!   a [`String`].
//! - [`HashObjectCommand`](hash_object::HashObjectCommand) returns the computed
//!   SHA.
//!
//! # Escape hatches
//!
//! Every command supports [`arg`](GitCommand::arg), [`args`](GitCommand::args),
//! [`flag`](GitCommand::flag), and [`option`](GitCommand::option). Raw args are
//! appended **after** the command's typed flags, so they compose naturally:
//!
//! ```no_run
//! # async fn ex() -> git_wrapper::Result<()> {
//! # use git_wrapper::{GitCommand, Repository};
//! let repo = Repository::open("/repo")?;
//! // `--shortstat` isn't on DiffCommand yet — fine, append it raw:
//! let out = repo.diff().cached().arg("--shortstat").execute().await?;
//! println!("{}", out.stdout);
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tracing::{debug, error, instrument, trace, warn};

pub mod add;
pub mod bisect;
pub mod branch;
pub mod cat_file;
pub mod checkout;
pub mod cherry_pick;
pub mod clone;
pub mod commit;
pub mod config;
pub mod describe;
pub mod diff;
pub mod fetch;
pub mod for_each_ref;
pub mod grep;
pub mod hash_object;
pub mod init;
pub mod log;
pub mod ls_files;
pub mod ls_tree;
pub mod merge;
pub mod mv;
pub mod pull;
pub mod push;
pub mod rebase;
pub mod reflog;
pub mod remote;
pub mod reset;
pub mod restore;
pub mod rev_parse;
pub mod rm;
pub mod show;
pub mod show_ref;
pub mod stash;
pub mod status;
pub mod submodule;
pub mod switch;
pub mod symbolic_ref;
pub mod tag;
pub mod update_ref;
pub mod worktree;

/// Default timeout applied when none is configured on the executor.
///
/// Set to `None` by default — callers opt in to timeouts explicitly.
pub const DEFAULT_COMMAND_TIMEOUT: Option<Duration> = None;

/// Trait implemented by every git subcommand wrapper.
#[async_trait]
pub trait GitCommand {
    /// The typed output produced by this command.
    type Output;

    /// Borrow the shared executor.
    fn get_executor(&self) -> &CommandExecutor;

    /// Mutably borrow the shared executor.
    fn get_executor_mut(&mut self) -> &mut CommandExecutor;

    /// Build the full argument vector (subcommand + flags + positionals)
    /// excluding the leading `git` program.
    fn build_command_args(&self) -> Vec<String>;

    /// Run the command and decode its output into [`Self::Output`].
    async fn execute(&self) -> Result<Self::Output>;

    /// Spawn `git` with the given arguments and return the raw output.
    ///
    /// Command implementations call this from `execute()` and then decode
    /// stdout into their typed output.
    async fn execute_raw(&self) -> Result<CommandOutput> {
        let args = self.build_command_args();
        self.get_executor().execute_command(args).await
    }

    /// Append a single raw argument.
    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.get_executor_mut().add_arg(arg);
        self
    }

    /// Append several raw arguments.
    fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.get_executor_mut().add_args(args);
        self
    }

    /// Append a `--flag` (or `-f` if a single character).
    fn flag(&mut self, flag: &str) -> &mut Self {
        self.get_executor_mut().add_flag(flag);
        self
    }

    /// Append a `--key value` pair.
    fn option(&mut self, key: &str, value: &str) -> &mut Self {
        self.get_executor_mut().add_option(key, value);
        self
    }

    /// Run `git` in the given working directory.
    fn current_dir<P: Into<PathBuf>>(&mut self, dir: P) -> &mut Self {
        self.get_executor_mut().cwd = Some(dir.into());
        self
    }

    /// Set an environment variable for this invocation.
    fn env<K: Into<OsString>, V: Into<OsString>>(&mut self, key: K, value: V) -> &mut Self {
        self.get_executor_mut().env.insert(key.into(), value.into());
        self
    }

    /// Cap execution time. On expiry the process is killed and
    /// [`Error::Timeout`] is returned.
    fn with_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.get_executor_mut().timeout = Some(timeout);
        self
    }

    /// Convenience: set timeout in whole seconds.
    fn with_timeout_secs(&mut self, seconds: u64) -> &mut Self {
        self.get_executor_mut().timeout = Some(Duration::from_secs(seconds));
        self
    }
}

/// Shared machinery used by every [`GitCommand`] to spawn `git`.
#[derive(Debug, Clone, Default)]
pub struct CommandExecutor {
    /// Raw arguments appended via the escape hatch.
    pub raw_args: Vec<String>,
    /// Working directory for the subprocess.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables.
    pub env: HashMap<OsString, OsString>,
    /// Optional execution timeout.
    pub timeout: Option<Duration>,
}

impl CommandExecutor {
    /// Create an empty executor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: set the working directory.
    #[must_use]
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    /// Builder: set an environment variable.
    #[must_use]
    pub fn with_env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Builder: set the timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Append a raw argument.
    pub fn add_arg<S: AsRef<OsStr>>(&mut self, arg: S) {
        self.raw_args
            .push(arg.as_ref().to_string_lossy().into_owned());
    }

    /// Append several raw arguments.
    pub fn add_args<I, S>(&mut self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for a in args {
            self.add_arg(a);
        }
    }

    /// Append a flag, normalizing to `-x` for single chars and `--word` otherwise.
    pub fn add_flag(&mut self, flag: &str) {
        let normalized = if flag.starts_with('-') {
            flag.to_string()
        } else if flag.len() == 1 {
            format!("-{flag}")
        } else {
            format!("--{flag}")
        };
        self.raw_args.push(normalized);
    }

    /// Append a `--key value` pair (or `-k value` for single chars).
    pub fn add_option(&mut self, key: &str, value: &str) {
        let normalized = if key.starts_with('-') {
            key.to_string()
        } else if key.len() == 1 {
            format!("-{key}")
        } else {
            format!("--{key}")
        };
        self.raw_args.push(normalized);
        self.raw_args.push(value.to_string());
    }

    /// Spawn `git` with `args` followed by any raw args, returning captured output.
    ///
    /// Non-zero exit codes become [`Error::CommandFailed`].
    #[instrument(
        name = "git.command",
        skip(self, args),
        fields(
            cwd = self.cwd.as_ref().map(|p| p.display().to_string()),
            timeout_secs = self.timeout.map(|t| t.as_secs()),
        )
    )]
    pub async fn execute_command(&self, args: Vec<String>) -> Result<CommandOutput> {
        let mut all_args = args;
        all_args.extend(self.raw_args.iter().cloned());

        trace!(args = ?all_args, "executing git command");

        let result = if let Some(t) = self.timeout {
            self.execute_with_timeout(&all_args, t).await
        } else {
            self.execute_internal(&all_args).await
        };

        match &result {
            Ok(output) => debug!(
                exit_code = output.exit_code,
                stdout_len = output.stdout.len(),
                stderr_len = output.stderr.len(),
                "command completed"
            ),
            Err(e) => error!(error = %e, "command failed"),
        }

        result
    }

    async fn execute_internal(&self, all_args: &[String]) -> Result<CommandOutput> {
        let mut cmd = TokioCommand::new("git");
        cmd.args(all_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &self.cwd {
            cmd.current_dir(dir);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }

        let output = cmd.output().await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::GitNotFound
            } else {
                Error::Io {
                    message: format!("failed to spawn git: {e}"),
                    source: e,
                }
            }
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        if !success {
            return Err(Error::command_failed(
                format!("git {}", all_args.join(" ")),
                exit_code,
                stdout,
                stderr,
            ));
        }

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_code,
            success,
        })
    }

    async fn execute_with_timeout(
        &self,
        all_args: &[String],
        timeout_duration: Duration,
    ) -> Result<CommandOutput> {
        match tokio::time::timeout(timeout_duration, self.execute_internal(all_args)).await {
            Ok(r) => r,
            Err(_) => {
                warn!(
                    timeout_secs = timeout_duration.as_secs(),
                    "command timed out"
                );
                Err(Error::timeout(timeout_duration.as_secs()))
            }
        }
    }
}

/// Captured output from running a git command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Exit code (`-1` if the process was terminated by a signal).
    pub exit_code: i32,
    /// Whether the process exited with status 0.
    pub success: bool,
}

impl CommandOutput {
    /// stdout split into lines.
    #[must_use]
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }

    /// stderr split into lines.
    #[must_use]
    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }

    /// stdout with trailing whitespace trimmed.
    #[must_use]
    pub fn stdout_trimmed(&self) -> &str {
        self.stdout.trim_end()
    }
}

/// Locate the `git` binary, returning [`Error::GitNotFound`] if missing.
///
/// Commands don't call this on every execution — tokio's `Command::new("git")`
/// already reports a helpful IO error we translate. This helper is for callers
/// that want to verify availability up front.
pub fn find_git() -> Result<PathBuf> {
    which::which("git").map_err(|_| Error::GitNotFound)
}

/// Run `git --version` and return the raw version string.
pub async fn git_version() -> Result<String> {
    let output = CommandExecutor::new()
        .execute_command(vec!["--version".into()])
        .await?;
    Ok(output.stdout_trimmed().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executor_args() {
        let mut e = CommandExecutor::new();
        e.add_arg("foo");
        e.add_args(["a", "b"]);
        e.add_flag("verbose");
        e.add_flag("v");
        e.add_option("name", "bar");
        assert_eq!(
            e.raw_args,
            vec!["foo", "a", "b", "--verbose", "-v", "--name", "bar"]
        );
    }

    #[test]
    fn executor_timeout_builder() {
        let e = CommandExecutor::new().timeout(Duration::from_secs(5));
        assert_eq!(e.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn command_output_helpers() {
        let o = CommandOutput {
            stdout: "a\nb\n".into(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };
        assert_eq!(o.stdout_lines(), vec!["a", "b"]);
        assert_eq!(o.stdout_trimmed(), "a\nb");
    }
}
