//! `git read-tree` — read tree information into the index.
use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
/// Builder for `git read-tree`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ReadTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Tree-ish arguments.
    pub trees: Vec<String>,
    /// Perform a merge.
    pub merge: bool,
    /// Reset unmerged entries.
    pub reset: bool,
    /// Update working-tree files.
    pub update: bool,
    /// Empty the index.
    pub empty: bool,
}
impl ReadTreeCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a tree-ish.
    pub fn tree(&mut self, v: impl Into<String>) -> &mut Self {
        self.trees.push(v.into());
        self
    }
    /// Enable `-m`.
    pub fn merge(&mut self) -> &mut Self {
        self.merge = true;
        self
    }
    /// Enable `--reset`.
    pub fn reset(&mut self) -> &mut Self {
        self.reset = true;
        self
    }
    /// Enable `-u`.
    pub fn update(&mut self) -> &mut Self {
        self.update = true;
        self
    }
    /// Enable `--empty`.
    pub fn empty(&mut self) -> &mut Self {
        self.empty = true;
        self
    }
}
#[async_trait]
impl GitCommand for ReadTreeCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["read-tree".into()];
        if self.merge {
            a.push("-m".into())
        }
        if self.reset {
            a.push("--reset".into())
        }
        if self.update {
            a.push("-u".into())
        }
        if self.empty {
            a.push("--empty".into())
        }
        a.extend(self.trees.iter().cloned());
        a
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
