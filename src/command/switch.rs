//! `git switch` — switch branches (modern successor to `checkout`).

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git switch`.
#[derive(Debug, Clone, Default)]
pub struct SwitchCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Target branch.
    pub target: Option<String>,
    /// `-c` create.
    pub create: Option<String>,
    /// `-C` force create.
    pub force_create: Option<String>,
    /// `--detach`.
    pub detach: bool,
    /// `--discard-changes`.
    pub discard_changes: bool,
    /// `--track`.
    pub track: bool,
    /// `--no-track`.
    pub no_track: bool,
    /// `--orphan`.
    pub orphan: Option<String>,
}

impl SwitchCommand {
    /// New `switch` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Target branch.
    pub fn target(&mut self, t: impl Into<String>) -> &mut Self {
        self.target = Some(t.into());
        self
    }

    /// Create a branch (`-c`).
    pub fn create(&mut self, name: impl Into<String>) -> &mut Self {
        self.create = Some(name.into());
        self
    }

    /// Force-create (`-C`).
    pub fn force_create(&mut self, name: impl Into<String>) -> &mut Self {
        self.force_create = Some(name.into());
        self
    }

    /// Detach HEAD.
    pub fn detach(&mut self) -> &mut Self {
        self.detach = true;
        self
    }

    /// Discard local changes.
    pub fn discard_changes(&mut self) -> &mut Self {
        self.discard_changes = true;
        self
    }

    /// `--track`.
    pub fn track(&mut self) -> &mut Self {
        self.track = true;
        self
    }

    /// `--no-track`.
    pub fn no_track(&mut self) -> &mut Self {
        self.no_track = true;
        self
    }

    /// Orphan branch.
    pub fn orphan(&mut self, name: impl Into<String>) -> &mut Self {
        self.orphan = Some(name.into());
        self
    }
}

#[async_trait]
impl GitCommand for SwitchCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["switch".to_string()];
        if self.detach {
            args.push("--detach".into());
        }
        if self.discard_changes {
            args.push("--discard-changes".into());
        }
        if self.track {
            args.push("--track".into());
        }
        if self.no_track {
            args.push("--no-track".into());
        }
        if let Some(o) = &self.orphan {
            args.push("--orphan".into());
            args.push(o.clone());
        }
        if let Some(b) = &self.create {
            args.push("-c".into());
            args.push(b.clone());
        }
        if let Some(b) = &self.force_create {
            args.push("-C".into());
            args.push(b.clone());
        }
        if let Some(t) = &self.target {
            args.push(t.clone());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
