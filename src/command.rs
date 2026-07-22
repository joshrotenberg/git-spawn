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
//!   a [`String`] (or raw bytes via
//!   [`execute_bytes`](cat_file::CatFileCommand::execute_bytes) for binary
//!   blobs).
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
use async_trait::async_trait;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tracing::{debug, error, instrument, trace, warn};

pub mod add;
pub mod am;
pub mod apply;
pub mod bisect;
pub mod branch;
pub mod cat_file;
pub mod checkout;
pub mod cherry;
pub mod cherry_pick;
pub mod clone;
pub mod commit;
pub mod config;
pub mod describe;
pub mod diff;
pub mod fetch;
pub mod for_each_ref;
pub mod format_patch;
pub mod grep;
pub mod hash_object;
pub mod init;
pub mod interpret_trailers;
pub mod log;
pub mod ls_files;
pub mod ls_tree;
pub mod merge;
pub mod mv;
pub mod notes;
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
pub mod verify_commit;
pub mod verify_tag;
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

    /// Build the configured `git` subprocess (args, cwd, env, process group).
    ///
    /// On Unix the child is placed in its own process group so a timeout can
    /// signal the whole group. git spawns children of its own (pack processes,
    /// credential/askpass helpers, hooks) that would be orphaned if we only
    /// killed the direct child. `kill_on_drop` is a belt-and-suspenders guard:
    /// if the child handle is dropped without an explicit kill, the direct
    /// child is still terminated rather than leaked.
    fn build_command(&self, all_args: &[String]) -> TokioCommand {
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

        // Run git as the leader of a new process group (pgid == child pid).
        #[cfg(unix)]
        cmd.process_group(0);

        cmd.kill_on_drop(true);
        cmd
    }

    /// Decode a finished process into [`CommandOutput`], mapping non-zero
    /// exits to [`Error::CommandFailed`].
    fn finish(&self, all_args: &[String], output: std::process::Output) -> Result<CommandOutput> {
        let stdout = output.stdout;
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        if !success {
            return Err(Error::command_failed(
                format!("git {}", all_args.join(" ")),
                exit_code,
                String::from_utf8_lossy(&stdout).into_owned(),
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

    async fn execute_internal(&self, all_args: &[String]) -> Result<CommandOutput> {
        let output = self
            .build_command(all_args)
            .output()
            .await
            .map_err(map_spawn_error)?;
        self.finish(all_args, output)
    }

    async fn execute_with_timeout(
        &self,
        all_args: &[String],
        timeout_duration: Duration,
    ) -> Result<CommandOutput> {
        let child = self
            .build_command(all_args)
            .spawn()
            .map_err(map_spawn_error)?;

        // Capture the pid (== process-group id on Unix) before `wait_with_output`
        // takes ownership of the child; we need it to signal the group on timeout.
        let pid = child.id();

        match tokio::time::timeout(timeout_duration, child.wait_with_output()).await {
            Ok(Ok(output)) => self.finish(all_args, output),
            Ok(Err(e)) => Err(Error::Io {
                message: format!("failed to run git: {e}"),
                source: e,
            }),
            Err(_) => {
                // The `wait_with_output` future has been dropped, so the direct
                // child is being killed via `kill_on_drop`. Also signal the whole
                // group to reap any grandchildren git spawned.
                if let Some(pid) = pid {
                    kill_process_group(pid);
                }
                warn!(
                    timeout_secs = timeout_duration.as_secs(),
                    "command timed out"
                );
                Err(Error::timeout(timeout_duration.as_secs()))
            }
        }
    }
}

/// Map a spawn error to [`Error::GitNotFound`] when the binary is missing,
/// otherwise to [`Error::Io`].
fn map_spawn_error(e: std::io::Error) -> Error {
    if e.kind() == std::io::ErrorKind::NotFound {
        Error::GitNotFound
    } else {
        Error::Io {
            message: format!("failed to spawn git: {e}"),
            source: e,
        }
    }
}

/// Kill the process group led by `pid`.
///
/// The executor spawns git with `process_group(0)`, so the child's pid is also
/// its process-group id. Signalling the negative pgid reaches every process in
/// the group, including the pack/credential/hook children git spawned.
#[cfg(unix)]
fn kill_process_group(pid: u32) {
    // A pid that overflows `i32` cannot name a real process group; skip it.
    let Ok(pgid) = i32::try_from(pid) else {
        return;
    };
    // SAFETY: `kill(2)` with a negative pid signals a process group. It has no
    // memory-safety implications; a stale pgid simply returns `ESRCH`.
    unsafe {
        libc::kill(-pgid, libc::SIGKILL);
    }
}

/// Windows fallback: there is no portable process-group kill here. The direct
/// child is terminated via `kill_on_drop` (best-effort `TerminateProcess`);
/// grandchildren are not tracked. A Job Object would be the complete fix.
#[cfg(not(unix))]
fn kill_process_group(_pid: u32) {}

/// Captured output from running a git command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Captured stdout as raw bytes.
    ///
    /// git output is not guaranteed to be valid UTF-8 — `cat-file` on a binary
    /// blob, paths under unusual encodings, and `-z`/NUL-delimited formats all
    /// produce bytes that lossy decoding would corrupt. The bytes are preserved
    /// verbatim; use [`stdout_str`](Self::stdout_str) for a lossy text view or
    /// [`stdout_bytes`](Self::stdout_bytes) for the raw slice.
    pub stdout: Vec<u8>,
    /// Captured stderr, decoded lossily as UTF-8 (git diagnostics are text).
    pub stderr: String,
    /// Exit code (`-1` if the process was terminated by a signal).
    pub exit_code: i32,
    /// Whether the process exited with status 0.
    pub success: bool,
}

impl CommandOutput {
    /// stdout as a raw byte slice. Use this for binary or non-UTF-8 output.
    #[must_use]
    pub fn stdout_bytes(&self) -> &[u8] {
        &self.stdout
    }

    /// stdout decoded as UTF-8, lossily (invalid sequences become U+FFFD).
    #[must_use]
    pub fn stdout_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }

    /// stdout decoded lossily and split into lines.
    #[must_use]
    pub fn stdout_lines(&self) -> Vec<String> {
        self.stdout_str().lines().map(ToOwned::to_owned).collect()
    }

    /// stderr split into lines.
    #[must_use]
    pub fn stderr_lines(&self) -> Vec<&str> {
        self.stderr.lines().collect()
    }

    /// stdout decoded lossily with trailing whitespace trimmed.
    #[must_use]
    pub fn stdout_trimmed(&self) -> String {
        self.stdout_str().trim_end().to_owned()
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
    Ok(output.stdout_trimmed())
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
            stdout: b"a\nb\n".to_vec(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        };
        assert_eq!(o.stdout_lines(), vec!["a", "b"]);
        assert_eq!(o.stdout_trimmed(), "a\nb");
        assert_eq!(o.stdout_bytes(), b"a\nb\n");
    }

    /// A timeout must reap the grandchildren git spawned, not just the direct
    /// child. Regression test for the process-group kill: a slow `pre-commit`
    /// hook backgrounds a `sleep`, records its pid, and we assert that pid is
    /// gone after the commit times out.
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
            matches!(err, Error::Timeout { .. }),
            "expected timeout, got {err:?}"
        );

        // The hook ran during the timeout window, so the pidfile exists.
        let grandchild: i32 = std::fs::read_to_string(&pidfile)
            .expect("hook should have written the grandchild pid")
            .trim()
            .parse()
            .expect("pidfile should contain a pid");

        // The group kill should reap the backgrounded sleep. Poll until the
        // pid is gone (kill(pid, 0) returns ESRCH); fail if it survives.
        let is_alive = |pid: i32| unsafe { libc::kill(pid, 0) == 0 };
        let deadline = Instant::now() + Duration::from_secs(5);
        while is_alive(grandchild) {
            assert!(
                Instant::now() < deadline,
                "grandchild pid {grandchild} survived the timeout: process group was not killed"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}
