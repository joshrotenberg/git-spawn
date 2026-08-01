//! `git mktree` — build a tree object from `ls-tree` formatted input.
use crate::command::{CommandExecutor, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
/// Builder for `git mktree`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MkTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Accept missing objects.
    pub missing: bool,
    /// NUL-terminated input.
    pub null_terminate: bool,
}
impl MkTreeCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Supply `ls-tree` formatted bytes on stdin.
    pub fn stdin(&mut self, v: impl Into<Vec<u8>>) -> &mut Self {
        self.executor.stdin = Some(v.into());
        self
    }
    /// Allow missing objects.
    pub fn missing(&mut self) -> &mut Self {
        self.missing = true;
        self
    }
    /// Use NUL termination.
    pub fn null_terminate(&mut self) -> &mut Self {
        self.null_terminate = true;
        self
    }
}
#[async_trait]
impl GitCommand for MkTreeCommand {
    type Output = String;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["mktree".into()];
        if self.missing {
            a.push("--missing".into())
        }
        if self.null_terminate {
            a.push("-z".into())
        }
        a
    }
    async fn execute(&self) -> Result<String> {
        Ok(self.execute_raw().await?.stdout_trimmed())
    }
}
