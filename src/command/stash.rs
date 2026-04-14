//! `git stash` — stash the changes in a dirty working directory away.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Actions supported by `git stash`.
#[derive(Debug, Clone)]
pub enum StashAction {
    /// `git stash push [-m <msg>]`.
    Push {
        /// Optional message.
        message: Option<String>,
        /// Include untracked files.
        include_untracked: bool,
        /// Keep the index intact.
        keep_index: bool,
    },
    /// `git stash pop [stash@{n}]`.
    Pop(Option<String>),
    /// `git stash apply [stash@{n}]`.
    Apply(Option<String>),
    /// `git stash drop [stash@{n}]`.
    Drop(Option<String>),
    /// `git stash list`.
    List,
    /// `git stash show [stash@{n}]`.
    Show(Option<String>),
    /// `git stash clear`.
    Clear,
}

/// Builder for `git stash`.
#[derive(Debug, Clone)]
pub struct StashCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: StashAction,
}

impl Default for StashCommand {
    fn default() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: StashAction::Push {
                message: None,
                include_untracked: false,
                keep_index: false,
            },
        }
    }
}

impl StashCommand {
    /// `stash push`.
    #[must_use]
    pub fn push() -> Self {
        Self::default()
    }

    /// Set push message.
    pub fn message(mut self, m: impl Into<String>) -> Self {
        if let StashAction::Push { message, .. } = &mut self.action {
            *message = Some(m.into());
        }
        self
    }

    /// Include untracked files.
    #[must_use]
    pub fn include_untracked(mut self) -> Self {
        if let StashAction::Push {
            include_untracked, ..
        } = &mut self.action
        {
            *include_untracked = true;
        }
        self
    }

    /// Keep index intact.
    #[must_use]
    pub fn keep_index(mut self) -> Self {
        if let StashAction::Push { keep_index, .. } = &mut self.action {
            *keep_index = true;
        }
        self
    }

    /// `stash pop`.
    #[must_use]
    pub fn pop(stash: Option<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: StashAction::Pop(stash),
        }
    }

    /// `stash apply`.
    #[must_use]
    pub fn apply(stash: Option<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: StashAction::Apply(stash),
        }
    }

    /// `stash drop`.
    #[must_use]
    pub fn drop_stash(stash: Option<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: StashAction::Drop(stash),
        }
    }

    /// `stash list`.
    #[must_use]
    pub fn list() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: StashAction::List,
        }
    }

    /// `stash show`.
    #[must_use]
    pub fn show(stash: Option<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: StashAction::Show(stash),
        }
    }

    /// `stash clear`.
    #[must_use]
    pub fn clear() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: StashAction::Clear,
        }
    }
}

#[async_trait]
impl GitCommand for StashCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["stash".to_string()];
        match &self.action {
            StashAction::Push {
                message,
                include_untracked,
                keep_index,
            } => {
                args.push("push".into());
                if *include_untracked {
                    args.push("--include-untracked".into());
                }
                if *keep_index {
                    args.push("--keep-index".into());
                }
                if let Some(m) = message {
                    args.push("-m".into());
                    args.push(m.clone());
                }
            }
            StashAction::Pop(s) => {
                args.push("pop".into());
                if let Some(s) = s {
                    args.push(s.clone());
                }
            }
            StashAction::Apply(s) => {
                args.push("apply".into());
                if let Some(s) = s {
                    args.push(s.clone());
                }
            }
            StashAction::Drop(s) => {
                args.push("drop".into());
                if let Some(s) = s {
                    args.push(s.clone());
                }
            }
            StashAction::List => args.push("list".into()),
            StashAction::Show(s) => {
                args.push("show".into());
                if let Some(s) = s {
                    args.push(s.clone());
                }
            }
            StashAction::Clear => args.push("clear".into()),
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
