//! `git for-each-ref` — output information on each ref matching a pattern.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git for-each-ref`.
#[derive(Debug, Clone, Default)]
pub struct ForEachRefCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Patterns to match (e.g. `"refs/heads/*"`).
    pub patterns: Vec<String>,
    /// `--format=<fmt>`.
    pub format: Option<String>,
    /// `--count=<n>`.
    pub count: Option<u32>,
    /// `--sort=<key>`.
    pub sort: Option<String>,
    /// `--contains`.
    pub contains: Option<String>,
    /// `--merged`.
    pub merged: Option<String>,
    /// `--no-merged`.
    pub no_merged: Option<String>,
    /// `--points-at`.
    pub points_at: Option<String>,
}

impl ForEachRefCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pattern.
    pub fn pattern(&mut self, p: impl Into<String>) -> &mut Self {
        self.patterns.push(p.into());
        self
    }

    /// Custom `--format`.
    pub fn format(&mut self, fmt: impl Into<String>) -> &mut Self {
        self.format = Some(fmt.into());
        self
    }

    /// `--count`.
    pub fn count(&mut self, n: u32) -> &mut Self {
        self.count = Some(n);
        self
    }

    /// `--sort`.
    pub fn sort(&mut self, key: impl Into<String>) -> &mut Self {
        self.sort = Some(key.into());
        self
    }

    /// `--contains`.
    pub fn contains(&mut self, c: impl Into<String>) -> &mut Self {
        self.contains = Some(c.into());
        self
    }

    /// `--merged`.
    pub fn merged(&mut self, c: impl Into<String>) -> &mut Self {
        self.merged = Some(c.into());
        self
    }

    /// `--no-merged`.
    pub fn no_merged(&mut self, c: impl Into<String>) -> &mut Self {
        self.no_merged = Some(c.into());
        self
    }

    /// `--points-at`.
    pub fn points_at(&mut self, c: impl Into<String>) -> &mut Self {
        self.points_at = Some(c.into());
        self
    }
}

#[async_trait]
impl GitCommand for ForEachRefCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["for-each-ref".to_string()];
        if let Some(f) = &self.format {
            args.push(format!("--format={f}"));
        }
        if let Some(n) = self.count {
            args.push(format!("--count={n}"));
        }
        if let Some(s) = &self.sort {
            args.push(format!("--sort={s}"));
        }
        if let Some(c) = &self.contains {
            args.push(format!("--contains={c}"));
        }
        if let Some(c) = &self.merged {
            args.push(format!("--merged={c}"));
        }
        if let Some(c) = &self.no_merged {
            args.push(format!("--no-merged={c}"));
        }
        if let Some(c) = &self.points_at {
            args.push(format!("--points-at={c}"));
        }
        args.extend(self.patterns.iter().cloned());
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
