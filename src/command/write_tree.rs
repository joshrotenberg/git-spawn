//! `git write-tree` — write the index as a tree object.
use crate::command::{CommandExecutor, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
/// Builder for `git write-tree`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WriteTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Optional path prefix.
    pub prefix: Option<String>,
}
impl WriteTreeCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Restrict the tree to a prefix.
    pub fn prefix(&mut self, v: impl Into<String>) -> &mut Self {
        self.prefix = Some(v.into());
        self
    }
}
#[async_trait]
impl GitCommand for WriteTreeCommand {
    type Output = String;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["write-tree".into()];
        if let Some(p) = &self.prefix {
            a.push(format!("--prefix={p}"))
        }
        a
    }
    async fn execute(&self) -> Result<String> {
        Ok(self.execute_raw().await?.stdout_trimmed())
    }
}
