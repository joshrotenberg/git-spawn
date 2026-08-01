//! `git diff-files` — compare working-tree files with the index.
use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
/// Builder for `git diff-files`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DiffFilesCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Show names and statuses.
    pub name_status: bool,
    /// NUL-terminate entries.
    pub null_terminate: bool,
    /// Quiet mode.
    pub quiet: bool,
    /// Path filters.
    pub paths: Vec<String>,
}
impl DiffFilesCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
    /// Enable `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
    /// Add a path filter.
    pub fn path(&mut self, v: impl Into<String>) -> &mut Self {
        self.paths.push(v.into());
        self
    }

    /// Run `--quiet` and report whether the working tree differs from the index.
    ///
    /// Git exits 0 when there are no differences and 1 when differences exist.
    /// Other statuses remain command failures. This method requires
    /// [`quiet`](Self::quiet), since status 1 has a different meaning otherwise.
    pub async fn execute_has_differences(&self) -> Result<bool> {
        if !self.quiet {
            return Err(Error::invalid_config(
                "diff-files: execute_has_differences requires --quiet",
            ));
        }
        let output = self
            .executor
            .execute_command_os_allowing(self.build_command_os_args(), &[0, 1])
            .await?;
        Ok(output.exit_code == 1)
    }
}
#[async_trait]
impl GitCommand for DiffFilesCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["diff-files".into()];
        if self.name_status {
            a.push("--name-status".into())
        }
        if self.null_terminate {
            a.push("-z".into())
        }
        if self.quiet {
            a.push("--quiet".into())
        }
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
