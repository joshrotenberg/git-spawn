//! `git verify-tag` — check the GPG or SSH signature on a tag.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git verify-tag`.
///
/// Verifies the signature on one or more annotated tags. Only signed tags
/// carry a signature: a lightweight tag, or an annotated tag created without
/// `-s`, fails verification.
///
/// # Exit status
///
/// `git verify-tag` exits non-zero when a tag is unsigned or its signature
/// does not validate, so a failed verification arrives as
/// [`Error::CommandFailed`] rather than a successful [`CommandOutput`] with a
/// negative verdict. The captured `stderr` carries git's explanation (the
/// verification report is written to stderr, not stdout).
#[derive(Debug, Clone, Default)]
pub struct VerifyTagCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The tags to verify, in the order they were added.
    pub tags: Vec<String>,
    /// `--raw`: print the verification report in machine-readable form.
    pub raw: bool,
    /// `-v`: also print the contents of the tag object.
    pub verbose: bool,
}

impl VerifyTagCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify `tag`. Call repeatedly to verify several tags.
    pub fn tag(&mut self, tag: impl Into<String>) -> &mut Self {
        self.tags.push(tag.into());
        self
    }

    /// Emit the raw gpg status output instead of the human-readable report.
    pub fn raw(&mut self) -> &mut Self {
        self.raw = true;
        self
    }

    /// Also print the contents of each verified tag object.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }
}

#[async_trait]
impl GitCommand for VerifyTagCommand {
    /// Raw output. The verification report is on `stderr`; `-v` puts the tag
    /// object on `stdout`.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["verify-tag".to_string()];
        if self.raw {
            args.push("--raw".into());
        }
        if self.verbose {
            args.push("-v".into());
        }
        args.extend(self.tags.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.tags.is_empty() {
            return Err(Error::invalid_config(
                "verify-tag requires at least one tag",
            ));
        }
        self.execute_raw().await
    }
}
