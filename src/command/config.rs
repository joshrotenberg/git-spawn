//! `git config` — get and set repository or global options.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Configuration scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigScope {
    /// `--local` (default for a repo).
    Local,
    /// `--global` (~/.gitconfig).
    Global,
    /// `--system` (system-wide).
    System,
    /// `--worktree`.
    Worktree,
}

/// Actions supported by `git config`.
#[derive(Debug, Clone)]
pub enum ConfigAction {
    /// Get a value.
    Get {
        /// Key, e.g. `"user.email"`.
        key: String,
    },
    /// Get all values for a multi-valued key.
    GetAll {
        /// Key.
        key: String,
    },
    /// Set a value.
    Set {
        /// Key.
        key: String,
        /// Value.
        value: String,
    },
    /// Unset a value.
    Unset {
        /// Key.
        key: String,
    },
    /// Unset all values for a key.
    UnsetAll {
        /// Key.
        key: String,
    },
    /// Add an additional value for a multi-valued key.
    Add {
        /// Key.
        key: String,
        /// Value.
        value: String,
    },
    /// List all config keys.
    List,
}

/// Builder for `git config`.
#[derive(Debug, Clone)]
pub struct ConfigCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: ConfigAction,
    /// Optional scope.
    pub scope: Option<ConfigScope>,
}

impl ConfigCommand {
    /// `config <key>` — get a value.
    pub fn get(key: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ConfigAction::Get { key: key.into() },
            scope: None,
        }
    }

    /// `config --get-all <key>`.
    pub fn get_all(key: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ConfigAction::GetAll { key: key.into() },
            scope: None,
        }
    }

    /// `config <key> <value>` — set a value.
    pub fn set(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ConfigAction::Set {
                key: key.into(),
                value: value.into(),
            },
            scope: None,
        }
    }

    /// `config --unset <key>`.
    pub fn unset(key: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ConfigAction::Unset { key: key.into() },
            scope: None,
        }
    }

    /// `config --unset-all <key>`.
    pub fn unset_all(key: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ConfigAction::UnsetAll { key: key.into() },
            scope: None,
        }
    }

    /// `config --add <key> <value>`.
    pub fn add(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ConfigAction::Add {
                key: key.into(),
                value: value.into(),
            },
            scope: None,
        }
    }

    /// `config --list`.
    #[must_use]
    pub fn list() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ConfigAction::List,
            scope: None,
        }
    }

    /// Limit to a particular scope.
    #[must_use]
    pub fn scope(mut self, s: ConfigScope) -> Self {
        self.scope = Some(s);
        self
    }
}

#[async_trait]
impl GitCommand for ConfigCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["config".to_string()];
        match self.scope {
            Some(ConfigScope::Local) => args.push("--local".into()),
            Some(ConfigScope::Global) => args.push("--global".into()),
            Some(ConfigScope::System) => args.push("--system".into()),
            Some(ConfigScope::Worktree) => args.push("--worktree".into()),
            None => {}
        }
        match &self.action {
            ConfigAction::Get { key } => args.push(key.clone()),
            ConfigAction::GetAll { key } => {
                args.push("--get-all".into());
                args.push(key.clone());
            }
            ConfigAction::Set { key, value } => {
                args.push(key.clone());
                args.push(value.clone());
            }
            ConfigAction::Unset { key } => {
                args.push("--unset".into());
                args.push(key.clone());
            }
            ConfigAction::UnsetAll { key } => {
                args.push("--unset-all".into());
                args.push(key.clone());
            }
            ConfigAction::Add { key, value } => {
                args.push("--add".into());
                args.push(key.clone());
                args.push(value.clone());
            }
            ConfigAction::List => args.push("--list".into()),
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        // `git config --get` returns exit 1 when the key is missing; surface
        // that as CommandFailed per our standard model.
        self.execute_raw().await
    }
}

impl ConfigCommand {
    /// Convenience: run the command and return the trimmed value for `get`.
    ///
    /// Returns [`Error::InvalidConfig`] if the action isn't `get` or `get_all`.
    pub async fn execute_value(&self) -> Result<String> {
        match self.action {
            ConfigAction::Get { .. } | ConfigAction::GetAll { .. } => {
                let out = self.execute_raw().await?;
                Ok(out.stdout_trimmed())
            }
            _ => Err(Error::invalid_config(
                "execute_value only applies to get / get-all actions",
            )),
        }
    }
}
