//! `git am` — apply patches from a mailbox and record them as commits.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// Builder for `git am`.
///
/// Applies mbox-style patches (as produced by
/// [`FormatPatchCommand`](crate::FormatPatchCommand)) and commits each one with
/// its recorded author and message. Reading a mailbox from stdin is not
/// modelled: this builder always passes mailbox paths on the command line.
///
/// `git am` stops and leaves the repository mid-application when a patch does
/// not apply. The three session controls — [`cont`](Self::cont),
/// [`skip`](Self::skip) and [`abort`](Self::abort) — drive that state, and each
/// one replaces the rest of the argument vector.
#[derive(Debug, Clone, Default)]
pub struct AmCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The mailbox files to apply, in the order they were added.
    pub mailboxes: Vec<PathBuf>,
    /// `--signoff`: add a `Signed-off-by` trailer to each commit.
    pub signoff: bool,
    /// `--3way`: fall back to a three-way merge when a patch does not apply cleanly.
    pub three_way: bool,
    /// `--keep-cr`: keep the trailing CR on lines ending in CRLF.
    pub keep_cr: bool,
    /// `-p<n>`: number of leading path components to strip.
    pub strip: Option<u32>,
    /// `--continue`: resume after the conflict was resolved and staged.
    pub cont: bool,
    /// `--skip`: drop the current patch and move to the next one.
    pub skip: bool,
    /// `--abort`: restore the branch to its pre-`am` state.
    pub abort: bool,
}

impl AmCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply the mailbox at `path`. Call repeatedly to apply several mailboxes.
    pub fn mailbox(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.mailboxes.push(path.into());
        self
    }

    /// Add a `Signed-off-by` trailer to each commit.
    pub fn signoff(&mut self) -> &mut Self {
        self.signoff = true;
        self
    }

    /// Fall back to a three-way merge when a patch does not apply cleanly.
    pub fn three_way(&mut self) -> &mut Self {
        self.three_way = true;
        self
    }

    /// Keep the trailing CR on lines ending in CRLF.
    pub fn keep_cr(&mut self) -> &mut Self {
        self.keep_cr = true;
        self
    }

    /// Strip `n` leading path components from every path in the patch.
    pub fn strip(&mut self, n: u32) -> &mut Self {
        self.strip = Some(n);
        self
    }

    /// Resume an interrupted `am` after staging the conflict resolution.
    pub fn cont(&mut self) -> &mut Self {
        self.cont = true;
        self
    }

    /// Drop the patch that stopped the session and continue with the next one.
    pub fn skip(&mut self) -> &mut Self {
        self.skip = true;
        self
    }

    /// Abort the session and restore the original branch state.
    pub fn abort(&mut self) -> &mut Self {
        self.abort = true;
        self
    }

    /// Whether one of `--continue` / `--skip` / `--abort` was requested.
    fn is_session_control(&self) -> bool {
        self.cont || self.skip || self.abort
    }
}

#[async_trait]
impl GitCommand for AmCommand {
    /// Raw output. `git am` reports progress on stdout and failures on stderr
    /// with a non-zero exit status.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["am".to_string()];
        if self.abort {
            args.push("--abort".into());
            return args;
        }
        if self.cont {
            args.push("--continue".into());
            return args;
        }
        if self.skip {
            args.push("--skip".into());
            return args;
        }
        if self.signoff {
            args.push("--signoff".into());
        }
        if self.three_way {
            args.push("--3way".into());
        }
        if self.keep_cr {
            args.push("--keep-cr".into());
        }
        if let Some(n) = self.strip {
            args.push(format!("-p{n}"));
        }
        for mailbox in &self.mailboxes {
            args.push(mailbox.display().to_string());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.mailboxes.is_empty() && !self.is_session_control() {
            return Err(Error::invalid_config(
                "am requires at least one mailbox, or --continue / --skip / --abort",
            ));
        }
        self.execute_raw().await
    }
}
