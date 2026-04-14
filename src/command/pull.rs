//! `git pull` — fetch from and integrate with another repository or a local branch.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git pull`.
#[derive(Debug, Clone, Default)]
pub struct PullCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Remote name.
    pub remote: Option<String>,
    /// Refspec.
    pub refspec: Option<String>,
    /// `--rebase` (optionally set to a merge strategy name like `true`/`merges`/`interactive`).
    pub rebase: Option<String>,
    /// `--no-rebase`.
    pub no_rebase: bool,
    /// `--ff-only`.
    pub ff_only: bool,
    /// `--no-ff`.
    pub no_ff: bool,
    /// `--all`.
    pub all: bool,
    /// `--tags`.
    pub tags: bool,
    /// `--autostash`.
    pub autostash: bool,
    /// `--quiet`.
    pub quiet: bool,
}

impl PullCommand {
    /// New `pull`.
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
    /// Enable `--rebase`.
    pub fn rebase(&mut self) -> &mut Self {
        self.rebase = Some(String::new());
        self
    }
    /// `--rebase=<mode>`.
    pub fn rebase_mode(&mut self, m: impl Into<String>) -> &mut Self {
        self.rebase = Some(m.into());
        self
    }
    /// `--no-rebase`.
    pub fn no_rebase(&mut self) -> &mut Self {
        self.no_rebase = true;
        self
    }
    /// `--ff-only`.
    pub fn ff_only(&mut self) -> &mut Self {
        self.ff_only = true;
        self
    }
    /// `--no-ff`.
    pub fn no_ff(&mut self) -> &mut Self {
        self.no_ff = true;
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
    /// `--autostash`.
    pub fn autostash(&mut self) -> &mut Self {
        self.autostash = true;
        self
    }
    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for PullCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["pull".to_string()];
        if let Some(r) = &self.rebase {
            if r.is_empty() {
                args.push("--rebase".into());
            } else {
                args.push(format!("--rebase={r}"));
            }
        }
        if self.no_rebase {
            args.push("--no-rebase".into());
        }
        if self.ff_only {
            args.push("--ff-only".into());
        }
        if self.no_ff {
            args.push("--no-ff".into());
        }
        if self.all {
            args.push("--all".into());
        }
        if self.tags {
            args.push("--tags".into());
        }
        if self.autostash {
            args.push("--autostash".into());
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
