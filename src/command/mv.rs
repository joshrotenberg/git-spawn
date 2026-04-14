//! `git mv` — move or rename a file, directory, or symlink.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git mv`.
#[derive(Debug, Clone, Default)]
pub struct MvCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Source path.
    pub source: Option<String>,
    /// Destination path.
    pub destination: Option<String>,
    /// `--force` / `-f`.
    pub force: bool,
    /// `-k` skip missing.
    pub skip_missing: bool,
    /// `--dry-run`.
    pub dry_run: bool,
    /// `--verbose`.
    pub verbose: bool,
}

impl MvCommand {
    /// Move `src` to `dst`.
    pub fn new(src: impl Into<String>, dst: impl Into<String>) -> Self {
        Self {
            source: Some(src.into()),
            destination: Some(dst.into()),
            ..Self::default()
        }
    }
    /// `--force`.
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }
    /// `-k`.
    pub fn skip_missing(&mut self) -> &mut Self {
        self.skip_missing = true;
        self
    }
    /// `--dry-run`.
    pub fn dry_run(&mut self) -> &mut Self {
        self.dry_run = true;
        self
    }
    /// `--verbose`.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }
}

#[async_trait]
impl GitCommand for MvCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["mv".to_string()];
        if self.force {
            args.push("--force".into());
        }
        if self.skip_missing {
            args.push("-k".into());
        }
        if self.dry_run {
            args.push("--dry-run".into());
        }
        if self.verbose {
            args.push("--verbose".into());
        }
        if let Some(s) = &self.source {
            args.push(s.clone());
        }
        if let Some(d) = &self.destination {
            args.push(d.clone());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        if self.source.is_none() || self.destination.is_none() {
            return Err(Error::invalid_config("mv requires source and destination"));
        }
        self.execute_raw().await
    }
}
