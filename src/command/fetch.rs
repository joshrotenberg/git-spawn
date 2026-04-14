//! `git fetch` — download objects and refs from another repository.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git fetch`.
#[derive(Debug, Clone, Default)]
pub struct FetchCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Remote name.
    pub remote: Option<String>,
    /// Refspec.
    pub refspec: Option<String>,
    /// `--all`.
    pub all: bool,
    /// `--tags`.
    pub tags: bool,
    /// `--no-tags`.
    pub no_tags: bool,
    /// `--prune`.
    pub prune: bool,
    /// `--depth`.
    pub depth: Option<u32>,
    /// `--unshallow`.
    pub unshallow: bool,
    /// `--quiet`.
    pub quiet: bool,
}

impl FetchCommand {
    /// New `fetch`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Remote.
    pub fn remote(&mut self, r: impl Into<String>) -> &mut Self {
        self.remote = Some(r.into());
        self
    }
    /// Refspec.
    pub fn refspec(&mut self, r: impl Into<String>) -> &mut Self {
        self.refspec = Some(r.into());
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
    /// `--no-tags`.
    pub fn no_tags(&mut self) -> &mut Self {
        self.no_tags = true;
        self
    }
    /// `--prune`.
    pub fn prune(&mut self) -> &mut Self {
        self.prune = true;
        self
    }
    /// `--depth`.
    pub fn depth(&mut self, d: u32) -> &mut Self {
        self.depth = Some(d);
        self
    }
    /// `--unshallow`.
    pub fn unshallow(&mut self) -> &mut Self {
        self.unshallow = true;
        self
    }
    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for FetchCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["fetch".to_string()];
        if self.all {
            args.push("--all".into());
        }
        if self.tags {
            args.push("--tags".into());
        }
        if self.no_tags {
            args.push("--no-tags".into());
        }
        if self.prune {
            args.push("--prune".into());
        }
        if let Some(d) = self.depth {
            args.push(format!("--depth={d}"));
        }
        if self.unshallow {
            args.push("--unshallow".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if let Some(r) = &self.remote {
            args.push(r.clone());
        }
        if let Some(r) = &self.refspec {
            args.push(r.clone());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
