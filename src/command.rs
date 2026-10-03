//! Command execution primitives.
//!
//! Every git subcommand wrapper is a struct that implements [`GitCommand`].
//! The trait gives each command:
//!
//! - [`execute()`](GitCommand::execute) — run and return a typed output
//! - [`execute_raw_unchecked()`](GitCommand::execute_raw_unchecked) — return
//!   captured output even when git exits non-zero
//! - [`arg()`](GitCommand::arg) / [`args()`](GitCommand::args) — append raw
//!   CLI arguments (escape hatch)
//! - [`global_arg()`](GitCommand::global_arg) /
//!   [`global_args()`](GitCommand::global_args) — prepend Git-global arguments
//! - [`with_timeout()`](GitCommand::with_timeout) — cap execution time
//! - [`current_dir()`](GitCommand::current_dir) / [`env()`](GitCommand::env) —
//!   control the subprocess environment
//! - [`stdin_bytes()`](GitCommand::stdin_bytes) — pipe exact bytes to stdin
//!
//! Under the hood, each command delegates to a shared [`CommandExecutor`] that
//! spawns `git` via [`tokio::process::Command`], captures stdout/stderr, and
//! maps non-zero exits to [`Error::CommandFailed`] by default.
//!
//! # Constructing commands
//!
//! Command builders are `#[non_exhaustive]`: their public fields may be read
//! and updated, but downstream crates cannot construct them with struct
//! literals. This lets a command gain support for another git option without
//! making that addition a breaking change. Create commands through their
//! documented constructors or a [`Repository`](crate::Repository) accessor,
//! then configure them with fluent builder methods:
//!
//! ```no_run
//! # use git_spawn::{GitCommand, Repository};
//! # async fn example() -> git_spawn::Result<()> {
//! let repo = Repository::open("/repo")?;
//! repo.add().all().path("src").execute().await?;
//! # Ok(())
//! # }
//! ```
//!
//! Repository accessors are provided for commands whose meaning depends on a
//! repository or working tree. Commands that are inherently standalone stay
//! available through their direct constructors: [`VersionCommand`](version::VersionCommand)
//! inspects the installed Git and intentionally has no
//! [`Repository`](crate::Repository) accessor. Hybrid commands support both
//! forms: [`Repository::ls_remote`](crate::Repository::ls_remote) scopes
//! configured-remote lookup, while [`LsRemoteCommand::remote`](ls_remote::LsRemoteCommand::remote)
//! can query a standalone URL or path directly. Likewise, full ref-name
//! validation is standalone, while
//! [`Repository::check_ref_format`](crate::Repository::check_ref_format)
//! scopes branch-mode reflog expansion.
//!
//! Struct-literal construction is intentionally unsupported:
//!
//! ```compile_fail
//! use git_spawn::AddCommand;
//!
//! let command = AddCommand {
//!     all: true,
//!     ..AddCommand::default()
//! };
//! ```
//!
//! Public command option and action enums are also non-exhaustive because git
//! can add formats, modes, and actions. Downstream matches must include a
//! wildcard arm.
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
//!   a [`String`] (or raw bytes via
//!   [`execute_bytes`](cat_file::CatFileCommand::execute_bytes) for binary
//!   blobs).
//! - [`HashObjectCommand`](hash_object::HashObjectCommand) returns the computed
//!   SHA.
//!
//! # Escape hatches
//!
//! Every command supports [`global_arg`](GitCommand::global_arg),
//! [`global_args`](GitCommand::global_args), [`arg`](GitCommand::arg),
//! [`args`](GitCommand::args), [`flag`](GitCommand::flag), and
//! [`option`](GitCommand::option). Git-global args are prepended **before** the
//! subcommand, while raw args are appended **after all** typed arguments,
//! including any `--` separator and paths. Raw options must precede path tails;
//! use the ordered executor when composing that complete argv yourself:
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! # use git_spawn::{GitCommand, Repository};
//! let repo = Repository::open("/repo")?;
//! // `--shortstat` isn't on DiffCommand yet — fine, append it raw:
//! let out = repo.diff().cached().arg("--shortstat").execute().await?;
//! println!("{}", out.stdout_str());
//! # Ok(())
//! # }
//! ```

use crate::error::{Error, Result};
use crate::execution::{CancellationToken, OutputLimits};
pub use crate::output::{CommandOutput, ProcessStatus};
use async_trait::async_trait;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command as TokioCommand;
use tracing::{debug, error, trace};

pub mod add;
pub mod am;
pub mod apply;
pub mod archive;
pub mod bisect;
pub mod blame;
pub mod branch;
pub mod bundle;
pub mod cat_file;
pub mod check_attr;
pub mod check_ignore;
pub mod check_ref_format;
pub mod checkout;
pub mod cherry;
pub mod cherry_pick;
pub mod clean;
pub mod clone;
pub mod commit;
pub mod commit_tree;
pub mod config;
pub mod count_objects;
pub mod describe;
pub mod diff;
pub mod diff_files;
pub mod diff_index;
pub mod diff_tree;
pub mod fetch;
pub mod for_each_ref;
pub mod format_patch;
pub mod fsck;
pub mod gc;
pub mod grep;
pub mod hash_object;
pub mod init;
pub mod interpret_trailers;
pub mod log;
pub mod ls_files;
pub mod ls_remote;
pub mod ls_tree;
pub mod maintenance;
pub mod merge;
pub mod merge_base;
pub mod merge_file;
pub mod merge_tree;
pub mod mktree;
pub mod mv;
pub mod name_rev;
pub mod notes;
pub mod pull;
pub mod push;
pub mod range_diff;
pub mod read_tree;
pub mod rebase;
pub mod reflog;
pub mod remote;
pub mod rerere;
pub mod reset;
pub mod restore;
pub mod rev_list;
pub mod rev_parse;
pub mod revert;
pub mod rm;
pub mod shortlog;
pub mod show;
pub mod show_ref;
pub mod sparse_checkout;
pub mod stash;
pub mod status;
pub mod submodule;
pub mod switch;
pub mod symbolic_ref;
pub mod tag;
pub mod update_index;
pub mod update_ref;
pub mod var;
pub mod verify_commit;
pub mod verify_tag;
pub mod version;
pub mod worktree;
pub mod write_tree;

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

    /// Build the argument vector using OS-native values.
    ///
    /// The default keeps existing UTF-8 command builders ergonomic. Commands
    /// with typed [`PathBuf`] fields override this to preserve those paths.
    fn build_command_os_args(&self) -> Vec<OsString> {
        self.build_command_args()
            .into_iter()
            .map(OsString::from)
            .collect()
    }

    /// Run the command and decode its output into [`Self::Output`].
    async fn execute(&self) -> Result<Self::Output>;

    /// Spawn `git` with the given arguments and return the raw output.
    ///
    /// Command implementations call this from `execute()` and then decode
    /// stdout into their typed output.
    async fn execute_raw(&self) -> Result<CommandOutput> {
        let args = self.build_command_os_args();
        self.get_executor().execute_command_os(args).await
    }

    /// Spawn `git` and return its captured output without checking the exit status.
    ///
    /// Use this escape hatch for git plumbing whose documented non-zero statuses
    /// are ordinary control flow, such as `git diff --quiet --exit-code`. A
    /// normally completed process returns [`CommandOutput`] for every exit
    /// status. Spawn, I/O, and timeout failures are still returned as errors.
    ///
    /// Prefer [`execute`](Self::execute) or [`execute_raw`](Self::execute_raw)
    /// when every non-zero status represents a command failure.
    async fn execute_raw_unchecked(&self) -> Result<CommandOutput> {
        let args = self.build_command_os_args();
        self.get_executor().execute_command_os_unchecked(args).await
    }

    /// Append a single raw argument after all typed arguments, including paths.
    ///
    /// A raw flag after a typed `--` is a pathspec, not an option. For commands
    /// mixing raw options and paths, supply the entire ordered raw tail or use
    /// [`CommandExecutor::execute_command_os_unchecked`].
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

    /// Prepend a single raw Git-global argument before the subcommand.
    ///
    /// Separate values, such as the value for `-C`, must be added separately
    /// so their ordering and OS-native representation are preserved.
    fn global_arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.get_executor_mut().add_global_arg(arg);
        self
    }

    /// Prepend several raw Git-global arguments before the subcommand.
    fn global_args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.get_executor_mut().add_global_args(args);
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
        self.get_executor_mut()
            .env
            .push((key.into(), Some(value.into())));
        self
    }

    /// Remove an inherited variable in this child only. Last update wins.
    fn env_remove(&mut self, key: impl Into<OsString>) -> &mut Self {
        self.get_executor_mut().env.push((key.into(), None));
        self
    }

    /// Cap execution time. Expiry returns [`Error::Execution`] with a timeout
    /// reason, partial capture, and cleanup observations; it does not undo Git.
    fn with_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.get_executor_mut().timeout = Some(timeout);
        self
    }

    /// Convenience: set timeout in whole seconds.
    fn with_timeout_secs(&mut self, seconds: u64) -> &mut Self {
        self.get_executor_mut().timeout = Some(Duration::from_secs(seconds));
        self
    }

    /// Supply owned bytes to the subprocess's stdin.
    ///
    /// Calling this with an empty value still configures a pipe, which is
    /// immediately closed after writing. Without explicit configuration stdin
    /// is null (EOF), for both timed and untimed execution.
    fn stdin_bytes(&mut self, bytes: impl Into<Vec<u8>>) -> &mut Self {
        self.get_executor_mut().stdin = StdinMode::Bytes(bytes.into());
        self
    }

    /// Supply EOF on stdin (the default).
    fn stdin_null(&mut self) -> &mut Self {
        self.get_executor_mut().stdin = StdinMode::Null;
        self
    }

    /// Explicitly inherit the parent's stdin.
    fn stdin_inherit(&mut self) -> &mut Self {
        self.get_executor_mut().stdin = StdinMode::Inherit;
        self
    }

    /// Cancel through this token, then await execution for cleanup evidence.
    fn cancellation_token(&mut self, token: CancellationToken) -> &mut Self {
        self.get_executor_mut().cancellation = Some(token);
        self
    }

    /// Bound each captured output stream while it is being read.
    fn output_limits(&mut self, limits: OutputLimits) -> &mut Self {
        self.get_executor_mut().output_limits = limits;
        self
    }

    /// Bound cleanup after an interruption, independently of execution time.
    fn cleanup_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.get_executor_mut().cleanup_timeout = timeout;
        self
    }
}

/// Explicit subprocess input policy. The default supplies EOF.
#[derive(Clone, Default)]
pub enum StdinMode {
    /// Connect stdin to the null device.
    #[default]
    Null,
    /// Inherit the parent's input descriptor.
    Inherit,
    /// Write these exact bytes and close the input pipe.
    Bytes(Vec<u8>),
}

impl std::fmt::Debug for StdinMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => f.write_str("Null"),
            Self::Inherit => f.write_str("Inherit"),
            Self::Bytes(bytes) => f.debug_struct("Bytes").field("len", &bytes.len()).finish(),
        }
    }
}

/// Shared machinery used by every [`GitCommand`] to spawn `git`.
#[derive(Clone)]
pub struct CommandExecutor {
    /// Ordered Git-global arguments inserted before the typed subcommand.
    pub global_args: Vec<OsString>,
    /// Raw arguments appended via the escape hatch.
    pub raw_args: Vec<OsString>,
    /// Working directory for the subprocess.
    pub cwd: Option<PathBuf>,
    /// Ordered child-only environment updates: `None` removes a variable.
    /// The last update wins using the platform's environment-key semantics.
    pub env: Vec<(OsString, Option<OsString>)>,
    /// Optional execution timeout.
    pub timeout: Option<Duration>,
    /// Input policy, defaulting to EOF.
    pub stdin: StdinMode,
    /// Optional explicit cancellation token.
    pub cancellation: Option<CancellationToken>,
    /// Capture budgets; unlimited unless configured by the caller.
    pub output_limits: OutputLimits,
    /// Maximum time to await cleanup after interruption (default five seconds).
    pub cleanup_timeout: Duration,
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self {
            global_args: Vec::new(),
            raw_args: Vec::new(),
            cwd: None,
            env: Vec::new(),
            timeout: DEFAULT_COMMAND_TIMEOUT,
            stdin: StdinMode::Null,
            cancellation: None,
            output_limits: OutputLimits::default(),
            cleanup_timeout: Duration::from_secs(5),
        }
    }
}

impl std::fmt::Debug for CommandExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandExecutor")
            .field("global_argument_count", &self.global_args.len())
            .field("raw_argument_count", &self.raw_args.len())
            .field("has_cwd", &self.cwd.is_some())
            .field("environment_update_count", &self.env.len())
            .field("timeout", &self.timeout)
            .field("stdin", &self.stdin)
            .field("has_cancellation_token", &self.cancellation.is_some())
            .field("output_limits", &self.output_limits)
            .field("cleanup_timeout", &self.cleanup_timeout)
            .finish()
    }
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
        self.env.push((key.into(), Some(value.into())));
        self
    }

    /// Builder: remove a variable in the child without changing the parent.
    #[must_use]
    pub fn env_remove(mut self, key: impl Into<OsString>) -> Self {
        self.env.push((key.into(), None));
        self
    }

    /// Builder: set the timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builder: supply owned bytes to the subprocess's stdin.
    #[must_use]
    pub fn stdin_bytes(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.stdin = StdinMode::Bytes(bytes.into());
        self
    }

    /// Builder: supply EOF on stdin (the default).
    #[must_use]
    pub fn stdin_null(mut self) -> Self {
        self.stdin = StdinMode::Null;
        self
    }

    /// Builder: explicitly inherit the parent's stdin.
    #[must_use]
    pub fn stdin_inherit(mut self) -> Self {
        self.stdin = StdinMode::Inherit;
        self
    }

    /// Builder: request cancellation through a token, then await this call.
    #[must_use]
    pub fn cancellation_token(mut self, token: CancellationToken) -> Self {
        self.cancellation = Some(token);
        self
    }

    /// Builder: bound captured stdout and stderr independently.
    #[must_use]
    pub fn output_limits(mut self, limits: OutputLimits) -> Self {
        self.output_limits = limits;
        self
    }

    /// Builder: bound cleanup after cancellation, timeout, or I/O failure.
    #[must_use]
    pub fn cleanup_timeout(mut self, timeout: Duration) -> Self {
        self.cleanup_timeout = timeout;
        self
    }

    /// Append a raw argument.
    pub fn add_arg<S: AsRef<OsStr>>(&mut self, arg: S) {
        self.raw_args.push(arg.as_ref().to_owned());
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

    /// Add a Git-global argument before the typed subcommand.
    pub fn add_global_arg<S: AsRef<OsStr>>(&mut self, arg: S) {
        self.global_args.push(arg.as_ref().to_owned());
    }

    /// Add several ordered Git-global arguments before the typed subcommand.
    pub fn add_global_args<I, S>(&mut self, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.add_global_arg(arg);
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
        self.raw_args.push(normalized.into());
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
        self.raw_args.push(normalized.into());
        self.raw_args.push(value.into());
    }

    /// Spawn `git` with global args, `args`, then trailing raw args.
    ///
    /// Non-zero exit codes become [`Error::CommandFailed`].
    pub async fn execute_command(&self, args: Vec<String>) -> Result<CommandOutput> {
        self.execute_command_os(args.into_iter().map(OsString::from).collect())
            .await
    }

    /// Spawn `git` with OS-native global args, typed args, then raw args.
    ///
    /// Unlike rendered diagnostics, these values are passed to the operating
    /// system without Unicode conversion.
    pub async fn execute_command_os(&self, args: Vec<OsString>) -> Result<CommandOutput> {
        self.execute_command_os_allowing(args, &[0]).await
    }

    /// Execute a command while treating the listed exit codes as expected.
    pub(crate) async fn execute_command_os_allowing(
        &self,
        args: Vec<OsString>,
        allowed_exit_codes: &[i32],
    ) -> Result<CommandOutput> {
        self.execute_command_os_checked_by(args, |output| {
            allowed_exit_codes.contains(&output.exit_code)
        })
        .await
    }

    /// Execute a command and classify its output with a command-specific rule.
    pub(crate) async fn execute_command_os_checked_by<F>(
        &self,
        args: Vec<OsString>,
        is_expected: F,
    ) -> Result<CommandOutput>
    where
        F: FnOnce(&CommandOutput) -> bool,
    {
        let all_args = self.all_args(args);
        let output = self.execute_command_unchecked_inner(&all_args).await?;

        if !is_expected(&output) {
            // The raw command remains available programmatically. Error Display
            // and Debug deliberately omit arguments and captured bodies.
            let command = std::iter::once("git".into())
                .chain(all_args.iter().map(|arg| arg.to_string_lossy()))
                .collect::<Vec<_>>()
                .join(" ");
            return Err(Error::from_command_output(command, output));
        }

        Ok(output)
    }

    /// Spawn `git` with `args` followed by any raw args, returning captured
    /// output regardless of the process's exit status.
    ///
    /// This is intended for git plumbing that assigns meaning to non-zero exit
    /// statuses. A process that starts and completes normally always yields a
    /// [`CommandOutput`]; inspect [`CommandOutput::exit_code`] or
    /// [`CommandOutput::success`] to classify it. Spawn, I/O, and timeout
    /// failures remain errors.
    pub async fn execute_command_unchecked(&self, args: Vec<String>) -> Result<CommandOutput> {
        self.execute_command_os_unchecked(args.into_iter().map(OsString::from).collect())
            .await
    }

    /// Spawn `git` with OS-native arguments and return output for any exit status.
    pub async fn execute_command_os_unchecked(&self, args: Vec<OsString>) -> Result<CommandOutput> {
        let all_args = self.all_args(args);
        self.execute_command_unchecked_inner(&all_args).await
    }

    fn all_args(&self, args: Vec<OsString>) -> Vec<OsString> {
        self.global_args
            .iter()
            .cloned()
            .chain(args)
            .chain(self.raw_args.iter().cloned())
            .collect()
    }

    #[tracing::instrument(name = "git.command", skip_all, fields(
        argument_count = all_args.len(),
        timeout_ms = self.timeout.map(|duration| duration.as_millis() as u64),
    ))]
    async fn execute_command_unchecked_inner(
        &self,
        all_args: &[OsString],
    ) -> Result<CommandOutput> {
        trace!(argument_count = all_args.len(), "executing git command");

        let started = Instant::now();
        let input = match &self.stdin {
            StdinMode::Bytes(bytes) => Some(bytes.as_slice()),
            StdinMode::Null | StdinMode::Inherit => None,
        };
        let result = crate::execution::run(
            self.build_command(all_args),
            input,
            self.timeout,
            self.cancellation.as_ref(),
            self.output_limits,
            self.cleanup_timeout,
        )
        .await;

        match &result {
            Ok(output) => debug!(
                exit_code = output.exit_code,
                status = ?output.status,
                duration_ms = started.elapsed().as_millis() as u64,
                stdout_len = output.stdout.len(),
                stderr_len = output.stderr.len(),
                "command completed"
            ),
            Err(e) => {
                error!(category = e.category(), error = %e, duration_ms = started.elapsed().as_millis() as u64, "command failed")
            }
        }

        result
    }

    /// Build the configured `git` subprocess (args, cwd, env, process group).
    ///
    /// On Unix the child is placed in its own process group so a timeout can
    /// signal the whole group. git spawns children of its own (pack processes,
    /// credential/askpass helpers, hooks) that would be orphaned if we only
    /// killed the direct child. On Windows, commands are spawned
    /// suspended, assigned to a kill-on-close Job Object, and then resumed.
    /// `kill_on_drop` is a
    /// belt-and-suspenders guard:
    /// if the child handle is dropped without an explicit kill, the direct
    /// child is still terminated rather than leaked.
    fn build_command(&self, all_args: &[OsString]) -> TokioCommand {
        let mut cmd = TokioCommand::new("git");
        cmd.args(all_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        cmd.stdin(match self.stdin {
            StdinMode::Null => Stdio::null(),
            StdinMode::Inherit => Stdio::inherit(),
            StdinMode::Bytes(_) => Stdio::piped(),
        });

        if let Some(dir) = &self.cwd {
            cmd.current_dir(dir);
        }
        for (key, value) in &self.env {
            match value {
                Some(value) => {
                    cmd.env(key, value);
                }
                None => {
                    cmd.env_remove(key);
                }
            }
        }

        // Run git as the leader of a new process group (pgid == child pid).
        #[cfg(unix)]
        cmd.process_group(0);

        cmd.kill_on_drop(true);
        cmd
    }
}

/// Locate the `git` binary, returning [`Error::GitNotFound`] if missing.
///
/// Commands don't call this on every execution — tokio's `Command::new("git")`
/// reports spawn errors as [`Error::Io`], since a missing working directory
/// can produce the same OS error as a missing executable. This helper is for callers
/// that want to verify availability up front.
pub fn find_git() -> Result<PathBuf> {
    which::which("git").map_err(|_| Error::GitNotFound)
}

/// Run `git --version` and return the raw version string.
pub async fn git_version() -> Result<String> {
    let output = CommandExecutor::new()
        .execute_command(vec!["--version".into()])
        .await?;
    Ok(output.stdout_trimmed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executor_args() {
        let mut e = CommandExecutor::new();
        e.add_global_arg("--no-optional-locks");
        e.add_global_args(["-C", "/repo"]);
        e.add_arg("foo");
        e.add_args(["a", "b"]);
        e.add_flag("verbose");
        e.add_flag("v");
        e.add_option("name", "bar");
        assert_eq!(
            e.global_args,
            vec!["--no-optional-locks", "-C", "/repo"]
                .into_iter()
                .map(OsString::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            e.raw_args,
            vec!["foo", "a", "b", "--verbose", "-v", "--name", "bar"]
                .into_iter()
                .map(OsString::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn executor_orders_global_typed_and_trailing_args() {
        let mut executor = CommandExecutor::new();
        executor.add_global_args(["--no-optional-locks", "-C", "/repo"]);
        executor.add_args(["--porcelain=v2", "--untracked-files=no"]);

        assert_eq!(
            executor.all_args(vec!["status".into(), "--short".into()]),
            [
                "--no-optional-locks",
                "-C",
                "/repo",
                "status",
                "--short",
                "--porcelain=v2",
                "--untracked-files=no",
            ]
            .into_iter()
            .map(OsString::from)
            .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn invocation_local_config_precedes_subcommand_without_persisting() {
        let key = "git-spawn.issue130-test";
        let mut command = config::ConfigCommand::get(key);
        command.global_args(["-c", "git-spawn.issue130-test=invocation-only"]);

        let output = command.execute().await.unwrap();
        assert_eq!(output.stdout_trimmed(), "invocation-only");

        let output = CommandExecutor::new()
            .execute_command_unchecked(vec!["config".into(), "--get".into(), key.into()])
            .await
            .unwrap();
        assert!(!output.success, "invocation-local config must not persist");
    }

    #[tokio::test]
    async fn no_optional_locks_and_dash_c_precede_subcommand() {
        let dir = tempfile::tempdir().unwrap();
        CommandExecutor::new()
            .cwd(dir.path())
            .execute_command(vec!["init".into(), "-q".into()])
            .await
            .unwrap();

        let mut command = status::StatusCommand::new();
        command
            .global_arg("--no-optional-locks")
            .global_args([OsString::from("-C"), dir.path().as_os_str().to_owned()])
            .arg("--porcelain=v2");
        let output = command.execute().await.unwrap();

        assert!(output.success);
        assert!(output.stdout_bytes().is_empty());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn raw_argument_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let dir = tempfile::tempdir().unwrap();
        let executor = CommandExecutor::new().cwd(dir.path());
        executor
            .execute_command(vec!["init".into(), "-q".into()])
            .await
            .unwrap();
        std::fs::write(dir.path().join(".gitignore"), b"*\n").unwrap();

        let filename = OsString::from_vec(b"native-\xff-path".to_vec());

        let mut command = check_ignore::CheckIgnoreCommand::new();
        command
            .current_dir(dir.path())
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.quotePath")
            .env("GIT_CONFIG_VALUE_0", "false")
            .arg("--")
            .arg(&filename);
        let output = command.execute_raw().await.unwrap();

        let mut expected = filename.as_os_str().as_bytes().to_vec();
        expected.push(b'\n');
        assert_eq!(output.stdout_bytes(), expected);
    }

    #[cfg(unix)]
    #[test]
    fn global_argument_preserves_non_utf8_bytes() {
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let native_path = OsString::from_vec(b"native-\xff-repo".to_vec());
        let mut executor = CommandExecutor::new();
        executor.add_global_args([OsString::from("-C"), native_path]);

        let args = executor.all_args(vec!["status".into()]);
        assert_eq!(args[1].as_os_str().as_bytes(), b"native-\xff-repo");
    }

    #[test]
    fn executor_timeout_builder() {
        let e = CommandExecutor::new().timeout(Duration::from_secs(5));
        assert_eq!(e.timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn executor_stdin_builder_distinguishes_empty_from_absent() {
        let absent = CommandExecutor::new();
        let empty = CommandExecutor::new().stdin_bytes(Vec::new());
        assert!(matches!(absent.stdin, StdinMode::Null));
        assert!(matches!(empty.stdin, StdinMode::Bytes(ref bytes) if bytes.is_empty()));
    }

    #[test]
    fn command_output_helpers() {
        let o = CommandOutput {
            stdout: b"a\nb\n".to_vec(),
            stderr: Vec::new(),
            status: ProcessStatus::Exited { code: 0 },
            exit_code: 0,
            success: true,
        };
        assert_eq!(o.stdout_lines(), vec!["a", "b"]);
        assert_eq!(o.stdout_trimmed(), "a\nb");
        assert_eq!(o.stdout_bytes(), b"a\nb\n");
    }

    #[tokio::test]
    async fn unchecked_preserves_nonzero_output_while_checked_rejects_it() {
        let dir = tempfile::tempdir().unwrap();
        let executor = CommandExecutor::new().cwd(dir.path());

        executor
            .execute_command(vec!["init".into(), "-q".into()])
            .await
            .unwrap();
        std::fs::write(dir.path().join("empty"), b"").unwrap();
        std::fs::write(dir.path().join("untracked"), b"contents").unwrap();

        let args = vec![
            "diff".into(),
            "--quiet".into(),
            "--exit-code".into(),
            "--no-index".into(),
            "empty".into(),
            "untracked".into(),
        ];
        let output = executor
            .execute_command_unchecked(args.clone())
            .await
            .unwrap();
        assert_eq!(output.exit_code, 1);
        assert!(!output.success);

        let error = executor.execute_command(args).await.unwrap_err();
        assert!(
            matches!(error, Error::CommandFailed { exit_code: 1, .. }),
            "expected status 1 to remain checked, got {error:?}"
        );
    }

    #[tokio::test]
    async fn git_command_unchecked_escape_hatch_preserves_nonzero_status() {
        let dir = tempfile::tempdir().unwrap();
        CommandExecutor::new()
            .cwd(dir.path())
            .execute_command(vec!["init".into(), "-q".into()])
            .await
            .unwrap();
        std::fs::write(dir.path().join("empty"), b"").unwrap();
        std::fs::write(dir.path().join("changed"), b"contents").unwrap();

        let mut command = diff::DiffCommand::new();
        command.current_dir(dir.path()).args([
            "--quiet",
            "--exit-code",
            "--no-index",
            "empty",
            "changed",
        ]);
        let output = command.execute_raw_unchecked().await.unwrap();

        assert_eq!(output.exit_code, 1);
        assert!(!output.success);
    }

    /// A timeout must terminate the grandchildren git spawned, not just the direct
    /// child. Regression test for the process-group kill: a slow `pre-commit`
    /// hook backgrounds a `sleep`, records its pid, and we assert that pid is
    /// is no longer executing after the commit times out. Only a process's
    /// parent (or an adopting reaper) can reap it on Unix.
    #[cfg(unix)]
    #[tokio::test]
    async fn timeout_kills_process_group() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::Instant;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();

        // Every setup command goes through the executor (the tokio runtime),
        // never std::process, to avoid the macOS SIGCHLD reaper race.
        let run = |args: Vec<&str>| {
            let owned: Vec<String> = args.into_iter().map(ToOwned::to_owned).collect();
            let cwd = path.to_path_buf();
            async move {
                CommandExecutor::new()
                    .cwd(cwd)
                    .execute_command(owned)
                    .await
                    .unwrap()
            }
        };

        // Use a controlled hooks dir: some environments set core.hooksPath
        // globally, which makes .git/hooks/* inert. Point git at our own dir.
        let hooks_dir = path.join("hooks-under-test");
        std::fs::create_dir(&hooks_dir).unwrap();

        run(vec!["init", "-q"]).await;
        run(vec!["config", "user.email", "test@example.com"]).await;
        run(vec!["config", "user.name", "Test"]).await;
        run(vec!["config", "commit.gpgsign", "false"]).await;
        run(vec![
            "config",
            "core.hooksPath",
            hooks_dir.to_str().unwrap(),
        ])
        .await;
        std::fs::write(path.join("file.txt"), "hi").unwrap();
        run(vec!["add", "."]).await;

        // A pre-commit hook that backgrounds a long sleep (the "grandchild")
        // and records its pid, then waits on it so git blocks past the timeout.
        let pidfile = path.join("grandchild.pid");
        let hook = hooks_dir.join("pre-commit");
        std::fs::write(
            &hook,
            format!(
                "#!/bin/sh\nsleep 300 &\necho $! > \"{}\"\nwait\n",
                pidfile.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

        // The commit blocks in the hook and must time out.
        let err = CommandExecutor::new()
            .cwd(path)
            .timeout(Duration::from_millis(1500))
            .execute_command(vec!["commit".into(), "-m".into(), "x".into()])
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Execution { ref failure } if matches!(failure.kind, crate::execution::ExecutionFailureKind::TimedOut { .. })),
            "expected timeout, got {err:?}"
        );

        // The hook ran during the timeout window, so the pidfile exists.
        let grandchild: i32 = std::fs::read_to_string(&pidfile)
            .expect("hook should have written the grandchild pid")
            .trim()
            .parse()
            .expect("pidfile should contain a pid");

        // A container's PID 1 may leave terminated orphans as zombies. PID
        // existence alone must not be mistaken for continuing execution.
        let is_alive = |pid: i32| {
            #[cfg(target_os = "linux")]
            if let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                if let Some((_, rest)) = stat.rsplit_once(") ") {
                    if rest.starts_with('Z') || rest.starts_with('X') {
                        return false;
                    }
                }
            }
            // SAFETY: a zero signal queries this fixture PID's existence.
            unsafe { libc::kill(pid, 0) == 0 }
        };
        let deadline = Instant::now() + Duration::from_secs(5);
        while is_alive(grandchild) {
            assert!(
                Instant::now() < deadline,
                "grandchild pid {grandchild} survived the timeout: process group was not killed"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Git for Windows runs hooks through its bundled shell. This hook starts
    /// a separate long-lived Windows process, records its pid, and waits so the
    /// commit remains blocked until the executor timeout terminates the job.
    #[cfg(windows)]
    #[tokio::test]
    async fn timeout_kills_windows_job_descendants() {
        use std::time::Instant;
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let run = |args: Vec<&str>| {
            let owned: Vec<String> = args.into_iter().map(ToOwned::to_owned).collect();
            let cwd = path.to_path_buf();
            async move {
                CommandExecutor::new()
                    .cwd(cwd)
                    .execute_command(owned)
                    .await
                    .unwrap()
            }
        };

        let hooks_dir = path.join("hooks-under-test");
        std::fs::create_dir(&hooks_dir).unwrap();
        run(vec!["init", "-q"]).await;
        run(vec!["config", "user.email", "test@example.com"]).await;
        run(vec!["config", "user.name", "Test"]).await;
        run(vec!["config", "commit.gpgsign", "false"]).await;
        run(vec![
            "config",
            "core.hooksPath",
            hooks_dir.to_str().unwrap(),
        ])
        .await;
        std::fs::write(path.join("file.txt"), "hi").unwrap();
        run(vec!["add", "."]).await;

        let pidfile = path.join("grandchild.pid");
        let pidfile_arg = pidfile.to_string_lossy().replace('\\', "/");
        let helper = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        std::fs::write(
            hooks_dir.join("pre-commit"),
            format!(
                "#!/bin/sh\n\"{helper}\" command::tests::windows_detached_descendant_helper --ignored --exact\n"
            ),
        )
        .unwrap();

        let err = CommandExecutor::new()
            .cwd(path)
            .with_env("GIT_SPAWN_WINDOWS_DESCENDANT_PIDFILE", pidfile_arg)
            .timeout(Duration::from_secs(5))
            .execute_command(vec!["commit".into(), "-m".into(), "x".into()])
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Execution { ref failure } if matches!(failure.kind, crate::execution::ExecutionFailureKind::TimedOut { .. })),
            "expected timeout, got {err:?}"
        );

        let grandchild: u32 = std::fs::read_to_string(&pidfile)
            .expect("hook should have written the grandchild pid")
            .trim()
            .parse()
            .expect("pidfile should contain a pid");
        let is_alive = |pid| {
            // SAFETY: OpenProcess either returns a handle owned below or null.
            let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
            if process.is_null() {
                return false;
            }
            let mut code = 0;
            // SAFETY: process is a valid open handle and code is writable.
            let queried = unsafe { GetExitCodeProcess(process, &mut code) };
            // SAFETY: this closes exactly the handle returned by OpenProcess.
            unsafe { CloseHandle(process) };
            queried != 0 && code == STILL_ACTIVE as u32
        };

        let deadline = Instant::now() + Duration::from_secs(5);
        while is_alive(grandchild) {
            assert!(
                Instant::now() < deadline,
                "grandchild pid {grandchild} survived the timeout: Job Object was not terminated"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Exercise a descendant launched by Git's first command dispatch, rather
    /// than a hook reached after repository setup. The suspended launch must
    /// put Git in the Job Object before this alias can start PowerShell/ping.
    #[cfg(windows)]
    #[tokio::test]
    async fn timeout_contains_immediately_spawned_windows_descendant() {
        use std::time::Instant;
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("immediate-grandchild.pid");
        let pidfile_arg = pidfile.to_string_lossy().replace('\\', "/");
        let helper = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let alias = format!(
            "alias.spawn=!\"{helper}\" command::tests::windows_detached_descendant_helper --ignored --exact"
        );

        let mut executor = CommandExecutor::new()
            .cwd(dir.path())
            .with_env("GIT_SPAWN_WINDOWS_DESCENDANT_PIDFILE", pidfile_arg)
            .timeout(Duration::from_secs(5));
        executor.add_global_args([OsString::from("-c"), OsString::from(alias)]);
        let err = executor
            .execute_command(vec!["spawn".into()])
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Execution { ref failure } if matches!(failure.kind, crate::execution::ExecutionFailureKind::TimedOut { .. })),
            "expected timeout, got {err:?}"
        );

        let grandchild: u32 = std::fs::read_to_string(&pidfile)
            .expect("immediate alias should have written the grandchild pid")
            .trim()
            .parse()
            .expect("pidfile should contain a pid");
        let is_alive = |pid| {
            // SAFETY: OpenProcess either returns a handle owned below or null.
            let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
            if process.is_null() {
                return false;
            }
            let mut code = 0;
            // SAFETY: process is a valid open handle and code is writable.
            let queried = unsafe { GetExitCodeProcess(process, &mut code) };
            // SAFETY: this closes exactly the handle returned by OpenProcess.
            unsafe { CloseHandle(process) };
            queried != 0 && code == STILL_ACTIVE as u32
        };

        let deadline = Instant::now() + Duration::from_secs(5);
        while is_alive(grandchild) {
            assert!(
                Instant::now() < deadline,
                "immediately spawned descendant pid {grandchild} survived the timeout"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Keep an in-flight test invocation from surviving an assertion failure.
    #[cfg(windows)]
    struct WindowsAbortOnDrop(tokio::task::AbortHandle);

    #[cfg(windows)]
    impl Drop for WindowsAbortOnDrop {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    /// Hold a handle to the exact helper process so a reused PID cannot make
    /// the lifetime check pass or accidentally terminate a different process.
    #[cfg(windows)]
    struct WindowsObservedDescendant(windows_sys::Win32::Foundation::HANDLE);

    #[cfg(windows)]
    impl WindowsObservedDescendant {
        fn open(pid: u32) -> Self {
            use windows_sys::Win32::System::Threading::{
                OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
            };

            // SAFETY: this returns an owned process handle or null on failure.
            let handle = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
                    0,
                    pid,
                )
            };
            assert!(!handle.is_null(), "could not observe helper pid {pid}");
            Self(handle)
        }

        fn is_active(&self) -> bool {
            use windows_sys::Win32::Foundation::STILL_ACTIVE;
            use windows_sys::Win32::System::Threading::GetExitCodeProcess;

            let mut code = 0;
            // SAFETY: self owns a valid process handle and code is writable.
            assert_ne!(unsafe { GetExitCodeProcess(self.0, &mut code) }, 0);
            code == STILL_ACTIVE as u32
        }

        async fn wait_until_stopped(&self) {
            let deadline = Instant::now() + Duration::from_secs(5);
            while self.is_active() {
                assert!(
                    Instant::now() < deadline,
                    "Windows descendant survived execution interruption"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }

    #[cfg(windows)]
    impl Drop for WindowsObservedDescendant {
        fn drop(&mut self) {
            use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
            use windows_sys::Win32::System::Threading::{GetExitCodeProcess, TerminateProcess};

            let mut code = 0;
            // SAFETY: the handle is owned by this guard. On an assertion
            // failure, terminate a still-active helper before closing it.
            unsafe {
                if GetExitCodeProcess(self.0, &mut code) != 0 && code == STILL_ACTIVE as u32 {
                    TerminateProcess(self.0, 1);
                }
                CloseHandle(self.0);
            }
        }
    }

    #[cfg(windows)]
    async fn wait_for_windows_descendant_pid(pidfile: &std::path::Path) -> u32 {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(contents) = std::fs::read_to_string(pidfile) {
                if let Ok(pid) = contents.trim().parse() {
                    return pid;
                }
            }
            assert!(
                Instant::now() < deadline,
                "Windows helper did not publish {}",
                pidfile.display()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    #[cfg(windows)]
    async fn interrupt_untimed_windows_descendant(cancel: bool) {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("untimed-grandchild.pid");
        let pidfile_arg = pidfile.to_string_lossy().replace('\\', "/");
        let helper = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let alias = format!(
            "alias.spawn=!\"{helper}\" command::tests::windows_detached_descendant_helper --ignored --exact"
        );
        let token = CancellationToken::new();
        let mut executor = CommandExecutor::new()
            .cwd(dir.path())
            .with_env("GIT_SPAWN_WINDOWS_DESCENDANT_PIDFILE", pidfile_arg);
        assert!(
            executor.timeout.is_none(),
            "test requires an untimed command"
        );
        if cancel {
            executor = executor.cancellation_token(token.clone());
        }
        executor.add_global_args([OsString::from("-c"), OsString::from(alias)]);

        let mut task =
            tokio::spawn(async move { executor.execute_command(vec!["spawn".into()]).await });
        let _abort_on_panic = WindowsAbortOnDrop(task.abort_handle());
        let pid = wait_for_windows_descendant_pid(&pidfile).await;
        let descendant = WindowsObservedDescendant::open(pid);
        assert!(descendant.is_active(), "helper exited before interruption");
        assert!(
            !task.is_finished(),
            "git completed while helper was still active"
        );

        if cancel {
            token.cancel();
            let error = tokio::time::timeout(Duration::from_secs(5), &mut task)
                .await
                .expect("cancellation did not settle")
                .expect("execution task panicked")
                .unwrap_err();
            assert!(
                matches!(error, Error::Execution { ref failure }
                    if matches!(failure.kind, crate::execution::ExecutionFailureKind::Cancelled)
                    && failure.cleanup.direct_child_reaped),
                "expected handled cancellation, got {error:?}"
            );
        } else {
            task.abort();
            let join = tokio::time::timeout(Duration::from_secs(5), &mut task)
                .await
                .expect("aborted execution future did not settle");
            assert!(join.unwrap_err().is_cancelled());
        }

        descendant.wait_until_stopped().await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn untimed_cancellation_kills_windows_job_descendant() {
        interrupt_untimed_windows_descendant(true).await;
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn dropping_untimed_future_kills_windows_job_descendant() {
        interrupt_untimed_windows_descendant(false).await;
    }

    /// Long-lived process invoked by the success-path containment regression.
    #[cfg(windows)]
    #[test]
    #[ignore = "spawned by the Windows descendant-lifetime regression test"]
    fn windows_detached_descendant_helper() {
        let pidfile = std::env::var_os("GIT_SPAWN_WINDOWS_DESCENDANT_PIDFILE")
            .expect("helper requires its pidfile environment variable");
        std::fs::write(pidfile, std::process::id().to_string()).unwrap();
        std::thread::sleep(Duration::from_secs(300));
    }

    /// A configured timeout must not change successful-command semantics.
    /// Detached helpers may intentionally outlive Git and are only terminated
    /// when the timeout/error/cancellation cleanup paths keep the job armed.
    #[cfg(windows)]
    #[tokio::test]
    async fn successful_timed_command_preserves_detached_windows_descendant() {
        use std::time::Instant;
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
            TerminateProcess,
        };

        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("successful-grandchild.pid");
        let stdout = dir.path().join("successful-grandchild.stdout");
        let stderr = dir.path().join("successful-grandchild.stderr");
        let pidfile_arg = pidfile.to_string_lossy().replace('\\', "/");
        let stdout_arg = stdout.to_string_lossy().replace('\\', "/");
        let stderr_arg = stderr.to_string_lossy().replace('\\', "/");
        let helper = std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let alias = format!(
            "alias.spawn=!\"{helper}\" command::tests::windows_detached_descendant_helper --ignored --exact >\"{stdout_arg}\" 2>\"{stderr_arg}\" &"
        );

        let mut executor = CommandExecutor::new()
            .cwd(dir.path())
            .with_env("GIT_SPAWN_WINDOWS_DESCENDANT_PIDFILE", pidfile_arg)
            .timeout(Duration::from_secs(10));
        executor.add_global_args([OsString::from("-c"), OsString::from(alias)]);
        let output = executor
            .execute_command(vec!["spawn".into()])
            .await
            .expect("git alias should finish before its configured timeout");
        assert!(output.success);

        let pidfile_deadline = Instant::now() + Duration::from_secs(5);
        let mut last_pidfile_error;
        let grandchild: u32 = loop {
            match std::fs::read_to_string(&pidfile) {
                Ok(contents) => match contents.trim().parse() {
                    Ok(pid) => break pid,
                    Err(error) => {
                        last_pidfile_error =
                            format!("could not parse contents {contents:?} as a pid: {error}");
                    }
                },
                Err(error) => {
                    last_pidfile_error = format!("could not read pidfile: {error}");
                }
            }
            assert!(
                Instant::now() < pidfile_deadline,
                "detached helper did not publish a valid pidfile {} within 5 seconds; last error: {last_pidfile_error}",
                pidfile.display()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        // SAFETY: OpenProcess either returns a handle owned below or null.
        let process = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
                0,
                grandchild,
            )
        };
        assert!(
            !process.is_null(),
            "detached descendant pid {grandchild} should survive successful Git completion"
        );
        let mut code = 0;
        // SAFETY: process is a valid open handle and code is writable.
        let queried = unsafe { GetExitCodeProcess(process, &mut code) };
        assert_ne!(queried, 0, "descendant exit status should be queryable");
        assert_eq!(
            code, STILL_ACTIVE as u32,
            "detached descendant pid {grandchild} was killed after successful Git completion"
        );

        // SAFETY: process is a valid handle opened with PROCESS_TERMINATE.
        assert_ne!(unsafe { TerminateProcess(process, 1) }, 0);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            // SAFETY: process remains valid until CloseHandle below.
            assert_ne!(unsafe { GetExitCodeProcess(process, &mut code) }, 0);
            if code != STILL_ACTIVE as u32 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "test descendant pid {grandchild} did not terminate during cleanup"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // SAFETY: this closes exactly the handle returned by OpenProcess.
        unsafe { CloseHandle(process) };
    }
}
