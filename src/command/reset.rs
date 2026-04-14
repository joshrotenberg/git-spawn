//! `git reset` — reset current HEAD to the specified state.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Reset mode.
#[derive(Debug, Clone, Copy)]
pub enum ResetMode {
    /// `--soft`: move HEAD, leave index and working tree.
    Soft,
    /// `--mixed` (default): move HEAD and index, leave working tree.
    Mixed,
    /// `--hard`: move HEAD, index, and working tree.
    Hard,
    /// `--merge`.
    Merge,
    /// `--keep`.
    Keep,
}

/// Builder for `git reset`.
#[derive(Debug, Clone, Default)]
pub struct ResetCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Mode.
    pub mode: Option<ResetMode>,
    /// Target commit.
    pub commit: Option<String>,
    /// Pathspecs (for path-limited reset).
    pub paths: Vec<String>,
    /// `--quiet`.
    pub quiet: bool,
}

impl ResetCommand {
    /// New `reset`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set mode.
    pub fn mode(&mut self, m: ResetMode) -> &mut Self {
        self.mode = Some(m);
        self
    }

    /// Target commit.
    pub fn commit(&mut self, c: impl Into<String>) -> &mut Self {
        self.commit = Some(c.into());
        self
    }

    /// Restrict to paths.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }

    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for ResetCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["reset".to_string()];
        match self.mode {
            Some(ResetMode::Soft) => args.push("--soft".into()),
            Some(ResetMode::Mixed) => args.push("--mixed".into()),
            Some(ResetMode::Hard) => args.push("--hard".into()),
            Some(ResetMode::Merge) => args.push("--merge".into()),
            Some(ResetMode::Keep) => args.push("--keep".into()),
            None => {}
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if let Some(c) = &self.commit {
            args.push(c.clone());
        }
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
