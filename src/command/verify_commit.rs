//! `git verify-commit` — check the GPG or SSH signature on a commit.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git verify-commit`.
///
/// Verifies the signature recorded on one or more commits. The signature
/// itself is produced at commit time; see [`SigningOps`](crate::signing) for
/// the configuration that governs it.
///
/// # Exit status
///
/// `git verify-commit` exits non-zero when a commit is unsigned or its
/// signature does not validate, so a failed verification arrives as
/// [`Error::CommandFailed`] rather than a successful [`CommandOutput`] with a
/// negative verdict. The captured `stderr` carries git's explanation (the
/// verification report is written to stderr, not stdout).
#[derive(Debug, Clone, Default)]
pub struct VerifyCommitCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The commits to verify, in the order they were added.
    pub commits: Vec<String>,
    /// `--raw`: print the verification report in machine-readable form.
    pub raw: bool,
    /// `-v`: also print the contents of the commit object.
    pub verbose: bool,
}

impl VerifyCommitCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify `commit`. Call repeatedly to verify several commits.
    pub fn commit(&mut self, commit: impl Into<String>) -> &mut Self {
        self.commits.push(commit.into());
        self
    }

    /// Emit the raw gpg status output instead of the human-readable report.
    pub fn raw(&mut self) -> &mut Self {
        self.raw = true;
        self
    }

    /// Also print the contents of each verified commit object.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }
}

#[async_trait]
impl GitCommand for VerifyCommitCommand {
    /// Raw output. The verification report is on `stderr`; `-v` puts the
    /// commit object on `stdout`.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["verify-commit".to_string()];
        if self.raw {
            args.push("--raw".into());
        }
        if self.verbose {
            args.push("-v".into());
        }
        args.extend(self.commits.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.commits.is_empty() {
            return Err(Error::invalid_config(
                "verify-commit requires at least one commit",
            ));
        }
        self.execute_raw().await
    }
}
