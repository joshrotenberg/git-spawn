//! `git push` — update remote refs along with associated objects.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git push`.
#[derive(Debug, Clone, Default)]
pub struct PushCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Remote name.
    pub remote: Option<String>,
    /// Refspecs.
    pub refspecs: Vec<String>,
    /// `--all`.
    pub all: bool,
    /// `--tags`.
    pub tags: bool,
    /// `--follow-tags`.
    pub follow_tags: bool,
    /// `--force` / `-f`.
    pub force: bool,
    /// `--force-with-lease`.
    pub force_with_lease: bool,
    /// `--delete`.
    pub delete: bool,
    /// `--set-upstream` / `-u`.
    pub set_upstream: bool,
    /// `--dry-run` / `-n`.
    pub dry_run: bool,
    /// `--atomic`.
    pub atomic: bool,
    /// `--quiet`.
    pub quiet: bool,
}

impl PushCommand {
    /// New `push`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Remote.
    pub fn remote(&mut self, r: impl Into<String>) -> &mut Self {
        self.remote = Some(r.into());
        self
    }
    /// Add a refspec.
    pub fn refspec(&mut self, r: impl Into<String>) -> &mut Self {
        self.refspecs.push(r.into());
        self
    }
    /// `--all`.
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }
    /// `--tags`.
    pub fn tags(&mut self) -> &mut Self {
        self.tags = true;
        self
    }
    /// `--follow-tags`.
    pub fn follow_tags(&mut self) -> &mut Self {
        self.follow_tags = true;
        self
    }
    /// `--force`.
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }
    /// `--force-with-lease`.
    pub fn force_with_lease(&mut self) -> &mut Self {
        self.force_with_lease = true;
        self
    }
    /// `--delete`.
    pub fn delete(&mut self) -> &mut Self {
        self.delete = true;
        self
    }
    /// `-u` / `--set-upstream`.
    pub fn set_upstream(&mut self) -> &mut Self {
        self.set_upstream = true;
        self
    }
    /// `--dry-run`.
    pub fn dry_run(&mut self) -> &mut Self {
        self.dry_run = true;
        self
    }
    /// `--atomic`.
    pub fn atomic(&mut self) -> &mut Self {
        self.atomic = true;
        self
    }
    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for PushCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["push".to_string()];
        if self.all {
            args.push("--all".into());
        }
        if self.tags {
            args.push("--tags".into());
        }
        if self.follow_tags {
            args.push("--follow-tags".into());
        }
        if self.force {
            args.push("--force".into());
        }
        if self.force_with_lease {
            args.push("--force-with-lease".into());
        }
        if self.delete {
            args.push("--delete".into());
        }
        if self.set_upstream {
            args.push("--set-upstream".into());
        }
        if self.dry_run {
            args.push("--dry-run".into());
        }
        if self.atomic {
            args.push("--atomic".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if let Some(r) = &self.remote {
            args.push(r.clone());
        }
        args.extend(self.refspecs.iter().cloned());
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
