//! `git checkout` — switch branches or restore working tree files.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git checkout`.
#[derive(Debug, Clone, Default)]
pub struct CheckoutCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Target branch, commit, or tree-ish.
    pub target: Option<String>,
    /// `-b` create branch.
    pub create: Option<String>,
    /// `-B` create/reset branch.
    pub create_or_reset: Option<String>,
    /// `--force` / `-f`.
    pub force: bool,
    /// `--track`.
    pub track: bool,
    /// `--no-track`.
    pub no_track: bool,
    /// `--orphan`.
    pub orphan: Option<String>,
    /// `--detach`.
    pub detach: bool,
    /// Pathspecs.
    pub paths: Vec<String>,
    /// `--quiet`.
    pub quiet: bool,
}

impl CheckoutCommand {
    /// New `checkout` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch to the named branch/commit.
    pub fn target(&mut self, t: impl Into<String>) -> &mut Self {
        self.target = Some(t.into());
        self
    }

    /// Create a new branch (`-b`).
    pub fn create(&mut self, name: impl Into<String>) -> &mut Self {
        self.create = Some(name.into());
        self
    }

    /// Create or reset a branch (`-B`).
    pub fn create_or_reset(&mut self, name: impl Into<String>) -> &mut Self {
        self.create_or_reset = Some(name.into());
        self
    }

    /// `--force`.
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }

    /// `--track`.
    pub fn track(&mut self) -> &mut Self {
        self.track = true;
        self
    }

    /// `--no-track`.
    pub fn no_track(&mut self) -> &mut Self {
        self.no_track = true;
        self
    }

    /// Create an orphan branch.
    pub fn orphan(&mut self, name: impl Into<String>) -> &mut Self {
        self.orphan = Some(name.into());
        self
    }

    /// Detach HEAD.
    pub fn detach(&mut self) -> &mut Self {
        self.detach = true;
        self
    }

    /// Restore a path from the index or tree.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }

    /// Suppress output.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for CheckoutCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["checkout".to_string()];
        if self.force {
            args.push("--force".into());
        }
        if self.track {
            args.push("--track".into());
        }
        if self.no_track {
            args.push("--no-track".into());
        }
        if self.detach {
            args.push("--detach".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if let Some(o) = &self.orphan {
            args.push("--orphan".into());
            args.push(o.clone());
        }
        if let Some(b) = &self.create {
            args.push("-b".into());
            args.push(b.clone());
        }
        if let Some(b) = &self.create_or_reset {
            args.push("-B".into());
            args.push(b.clone());
        }
        if let Some(t) = &self.target {
            args.push(t.clone());
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
