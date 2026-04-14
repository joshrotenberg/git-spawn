//! `git restore` — restore working tree files.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git restore`.
#[derive(Debug, Clone, Default)]
pub struct RestoreCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Pathspecs.
    pub paths: Vec<String>,
    /// `--source`.
    pub source: Option<String>,
    /// `--staged`.
    pub staged: bool,
    /// `--worktree`.
    pub worktree: bool,
    /// `--quiet`.
    pub quiet: bool,
    /// `--ours`.
    pub ours: bool,
    /// `--theirs`.
    pub theirs: bool,
}

impl RestoreCommand {
    /// New `restore`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }
    /// Source tree-ish.
    pub fn source(&mut self, s: impl Into<String>) -> &mut Self {
        self.source = Some(s.into());
        self
    }
    /// Restore the staged copy.
    pub fn staged(&mut self) -> &mut Self {
        self.staged = true;
        self
    }
    /// Restore the working tree copy.
    pub fn worktree(&mut self) -> &mut Self {
        self.worktree = true;
        self
    }
    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
    /// `--ours`.
    pub fn ours(&mut self) -> &mut Self {
        self.ours = true;
        self
    }
    /// `--theirs`.
    pub fn theirs(&mut self) -> &mut Self {
        self.theirs = true;
        self
    }
}

#[async_trait]
impl GitCommand for RestoreCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["restore".to_string()];
        if let Some(s) = &self.source {
            args.push(format!("--source={s}"));
        }
        if self.staged {
            args.push("--staged".into());
        }
        if self.worktree {
            args.push("--worktree".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if self.ours {
            args.push("--ours".into());
        }
        if self.theirs {
            args.push("--theirs".into());
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
