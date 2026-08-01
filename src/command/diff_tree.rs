//! `git diff-tree` — compare tree objects.
use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
/// Builder for `git diff-tree`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DiffTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Tree-ish arguments.
    pub trees: Vec<String>,
    /// Show names and statuses.
    pub name_status: bool,
    /// NUL-terminate entries.
    pub null_terminate: bool,
    /// Recurse into subtrees.
    pub recursive: bool,
    /// Path filters.
    pub paths: Vec<String>,
}
impl DiffTreeCommand {
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
    /// Enable `--name-status`.
    pub fn name_status(&mut self) -> &mut Self {
        self.name_status = true;
        self
    }
    /// Enable `-z`.
    pub fn null_terminate(&mut self) -> &mut Self {
        self.null_terminate = true;
        self
    }
    /// Enable `-r`.
    pub fn recursive(&mut self) -> &mut Self {
        self.recursive = true;
        self
    }
    /// Add a path filter.
    pub fn path(&mut self, v: impl Into<String>) -> &mut Self {
        self.paths.push(v.into());
        self
    }
}
#[async_trait]
impl GitCommand for DiffTreeCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["diff-tree".into()];
        if self.name_status {
            a.push("--name-status".into())
        }
        if self.null_terminate {
            a.push("-z".into())
        }
        if self.recursive {
            a.push("-r".into())
        }
        a.extend(self.trees.iter().cloned());
        if !self.paths.is_empty() {
            a.push("--".into());
            a.extend(self.paths.iter().cloned())
        }
        a
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
