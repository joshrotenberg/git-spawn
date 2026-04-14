//! `git show` — show various types of objects.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git show`.
#[derive(Debug, Clone, Default)]
pub struct ShowCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Objects to show.
    pub objects: Vec<String>,
    /// `--format=...`.
    pub format: Option<String>,
    /// `--stat`.
    pub stat: bool,
    /// `--name-only`.
    pub name_only: bool,
    /// `--no-patch`.
    pub no_patch: bool,
}

impl ShowCommand {
    /// New `show` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a specific object (commit, tag, tree, blob).
    pub fn object(&mut self, o: impl Into<String>) -> &mut Self {
        self.objects.push(o.into());
        self
    }

    /// Pretty format.
    pub fn format(&mut self, fmt: impl Into<String>) -> &mut Self {
        self.format = Some(fmt.into());
        self
    }

    /// Include `--stat`.
    pub fn stat(&mut self) -> &mut Self {
        self.stat = true;
        self
    }

    /// `--name-only`.
    pub fn name_only(&mut self) -> &mut Self {
        self.name_only = true;
        self
    }

    /// Suppress patch output.
    pub fn no_patch(&mut self) -> &mut Self {
        self.no_patch = true;
        self
    }
}

#[async_trait]
impl GitCommand for ShowCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["show".to_string()];
        if let Some(f) = &self.format {
            args.push(format!("--format={f}"));
        }
        if self.stat {
            args.push("--stat".into());
        }
        if self.name_only {
            args.push("--name-only".into());
        }
        if self.no_patch {
            args.push("--no-patch".into());
        }
        args.extend(self.objects.iter().cloned());
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
