//! `git commit-tree` — create a commit object from a tree.
use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
/// Builder for `git commit-tree`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct CommitTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Tree object ID.
    pub tree: Option<String>,
    /// Parent commit IDs.
    pub parents: Vec<String>,
    /// Commit message supplied with `-m`.
    pub message: Option<String>,
}
impl CommitTreeCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the tree object.
    pub fn tree(&mut self, v: impl Into<String>) -> &mut Self {
        self.tree = Some(v.into());
        self
    }
    /// Add a parent.
    pub fn parent(&mut self, v: impl Into<String>) -> &mut Self {
        self.parents.push(v.into());
        self
    }
    /// Set the message.
    pub fn message(&mut self, v: impl Into<String>) -> &mut Self {
        self.message = Some(v.into());
        self
    }
}
#[async_trait]
impl GitCommand for CommitTreeCommand {
    type Output = String;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["commit-tree".into()];
        if let Some(t) = &self.tree {
            a.push(t.clone())
        }
        for p in &self.parents {
            a.push("-p".into());
            a.push(p.clone())
        }
        if let Some(m) = &self.message {
            a.push("-m".into());
            a.push(m.clone())
        }
        a
    }
    async fn execute(&self) -> Result<String> {
        if self.tree.is_none() {
            return Err(Error::invalid_config("commit-tree requires a tree"));
        }
        Ok(self.execute_raw().await?.stdout_trimmed())
    }
}
