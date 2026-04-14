//! `git branch` — list, create, or delete branches.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git branch`.
#[derive(Debug, Clone, Default)]
pub struct BranchCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `-l` list.
    pub list: bool,
    /// `-a` include remotes.
    pub all: bool,
    /// `-r` remotes only.
    pub remotes: bool,
    /// `-v` verbose.
    pub verbose: bool,
    /// Branch to create or operate on.
    pub name: Option<String>,
    /// Start-point for creation.
    pub start_point: Option<String>,
    /// `-d`.
    pub delete: Option<String>,
    /// `-D` force delete.
    pub force_delete: bool,
    /// `-m`.
    pub rename_from: Option<String>,
    /// `-m` target.
    pub rename_to: Option<String>,
    /// `--track`.
    pub track: bool,
    /// `--no-track`.
    pub no_track: bool,
    /// `--set-upstream-to`.
    pub set_upstream_to: Option<String>,
    /// `--unset-upstream`.
    pub unset_upstream: bool,
    /// `--show-current`.
    pub show_current: bool,
    /// `--contains` filter.
    pub contains: Option<String>,
    /// `--merged` filter.
    pub merged: Option<String>,
}

impl BranchCommand {
    /// New `branch` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// List mode.
    pub fn list(&mut self) -> &mut Self {
        self.list = true;
        self
    }

    /// Include remote-tracking branches.
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }

    /// Remote-tracking branches only.
    pub fn remotes(&mut self) -> &mut Self {
        self.remotes = true;
        self
    }

    /// `-v`.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }

    /// Create a branch with this name.
    pub fn create(&mut self, name: impl Into<String>) -> &mut Self {
        self.name = Some(name.into());
        self
    }

    /// Start-point for `create`.
    pub fn start_point(&mut self, sp: impl Into<String>) -> &mut Self {
        self.start_point = Some(sp.into());
        self
    }

    /// Delete a branch (`-d`).
    pub fn delete(&mut self, name: impl Into<String>) -> &mut Self {
        self.delete = Some(name.into());
        self
    }

    /// Force delete.
    pub fn force_delete(&mut self) -> &mut Self {
        self.force_delete = true;
        self
    }

    /// Rename branch (`-m <old> <new>`).
    pub fn rename(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        self.rename_from = Some(from.into());
        self.rename_to = Some(to.into());
        self
    }

    /// Set upstream.
    pub fn set_upstream_to(&mut self, s: impl Into<String>) -> &mut Self {
        self.set_upstream_to = Some(s.into());
        self
    }

    /// Unset upstream.
    pub fn unset_upstream(&mut self) -> &mut Self {
        self.unset_upstream = true;
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

    /// `--show-current`.
    pub fn show_current(&mut self) -> &mut Self {
        self.show_current = true;
        self
    }

    /// Filter branches containing commit.
    pub fn contains(&mut self, c: impl Into<String>) -> &mut Self {
        self.contains = Some(c.into());
        self
    }

    /// Filter branches merged into commit.
    pub fn merged(&mut self, c: impl Into<String>) -> &mut Self {
        self.merged = Some(c.into());
        self
    }
}

#[async_trait]
impl GitCommand for BranchCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["branch".to_string()];
        if self.list {
            args.push("--list".into());
        }
        if self.all {
            args.push("--all".into());
        }
        if self.remotes {
            args.push("--remotes".into());
        }
        if self.verbose {
            args.push("--verbose".into());
        }
        if self.force_delete {
            args.push("-D".into());
        }
        if self.track {
            args.push("--track".into());
        }
        if self.no_track {
            args.push("--no-track".into());
        }
        if self.unset_upstream {
            args.push("--unset-upstream".into());
        }
        if self.show_current {
            args.push("--show-current".into());
        }
        if let Some(u) = &self.set_upstream_to {
            args.push(format!("--set-upstream-to={u}"));
        }
        if let Some(c) = &self.contains {
            args.push(format!("--contains={c}"));
        }
        if let Some(m) = &self.merged {
            args.push(format!("--merged={m}"));
        }
        if let Some(d) = &self.delete {
            args.push("-d".into());
            args.push(d.clone());
        } else if let (Some(from), Some(to)) = (&self.rename_from, &self.rename_to) {
            args.push("-m".into());
            args.push(from.clone());
            args.push(to.clone());
        } else if let Some(name) = &self.name {
            args.push(name.clone());
            if let Some(sp) = &self.start_point {
                args.push(sp.clone());
            }
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
