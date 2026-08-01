//! `git merge-tree` — perform a trial three-way merge.
use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
/// Builder for `git merge-tree`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MergeTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Explicit merge base for the legacy three-tree form.
    pub base: Option<String>,
    /// First tree or branch.
    pub ours: Option<String>,
    /// Second tree or branch.
    pub theirs: Option<String>,
    /// Write a real tree object and emit machine-readable merge information.
    pub write_tree: bool,
    /// Emit NUL-delimited output.
    pub null_terminate: bool,
    /// Show messages only.
    pub messages: bool,
}
impl MergeTreeCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Set an explicit merge base.
    pub fn base(&mut self, v: impl Into<String>) -> &mut Self {
        self.base = Some(v.into());
        self
    }
    /// Set the first side.
    pub fn ours(&mut self, v: impl Into<String>) -> &mut Self {
        self.ours = Some(v.into());
        self
    }
    /// Set the second side.
    pub fn theirs(&mut self, v: impl Into<String>) -> &mut Self {
        self.theirs = Some(v.into());
        self
    }
    /// Enable `--write-tree`.
    pub fn write_tree(&mut self) -> &mut Self {
        self.write_tree = true;
        self
    }
    /// Enable `-z`.
    pub fn null_terminate(&mut self) -> &mut Self {
        self.null_terminate = true;
        self
    }
    /// Enable `--messages`.
    pub fn messages(&mut self) -> &mut Self {
        self.messages = true;
        self
    }
}
#[async_trait]
impl GitCommand for MergeTreeCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["merge-tree".into()];
        if self.write_tree {
            a.push("--write-tree".into())
        }
        if self.null_terminate {
            a.push("-z".into())
        }
        if self.messages {
            a.push("--messages".into())
        }
        if let Some(v) = &self.base {
            a.push(v.clone())
        }
        if let Some(v) = &self.ours {
            a.push(v.clone())
        }
        if let Some(v) = &self.theirs {
            a.push(v.clone())
        }
        a
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
