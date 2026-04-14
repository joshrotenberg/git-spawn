//! `git rm` — remove files from the working tree and from the index.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git rm`.
#[derive(Debug, Clone, Default)]
pub struct RmCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Pathspecs.
    pub paths: Vec<String>,
    /// `--cached`.
    pub cached: bool,
    /// `-r`.
    pub recursive: bool,
    /// `--force`.
    pub force: bool,
    /// `--dry-run`.
    pub dry_run: bool,
    /// `--quiet`.
    pub quiet: bool,
}

impl RmCommand {
    /// New `rm`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }
    /// `--cached`.
    pub fn cached(&mut self) -> &mut Self {
        self.cached = true;
        self
    }
    /// `-r`.
    pub fn recursive(&mut self) -> &mut Self {
        self.recursive = true;
        self
    }
    /// `--force`.
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }
    /// `--dry-run`.
    pub fn dry_run(&mut self) -> &mut Self {
        self.dry_run = true;
        self
    }
    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for RmCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["rm".to_string()];
        if self.cached {
            args.push("--cached".into());
        }
        if self.recursive {
            args.push("-r".into());
        }
        if self.force {
            args.push("--force".into());
        }
        if self.dry_run {
            args.push("--dry-run".into());
        }
        if self.quiet {
            args.push("--quiet".into());
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
