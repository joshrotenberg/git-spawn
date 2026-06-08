//! `git symbolic-ref` — read or modify a symbolic ref (most commonly `HEAD`).

use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Actions supported by `git symbolic-ref`.
#[derive(Debug, Clone)]
pub enum SymbolicRefAction {
    /// Read the target of `ref` (e.g. `HEAD` -> `refs/heads/main`).
    Read {
        /// Ref name to read.
        name: String,
        /// `--short` shows the short form (`main`).
        short: bool,
    },
    /// Set `name` to point at `target`.
    Set {
        /// Ref name.
        name: String,
        /// Target ref.
        target: String,
        /// `-m <reason>` reflog message.
        reason: Option<String>,
    },
    /// Delete the symbolic ref.
    Delete {
        /// Ref name.
        name: String,
        /// `-q` suppress errors.
        quiet: bool,
    },
}

/// Builder for `git symbolic-ref`.
#[derive(Debug, Clone)]
pub struct SymbolicRefCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: SymbolicRefAction,
}

impl SymbolicRefCommand {
    /// Read the target of `name` (e.g. `read("HEAD")`).
    pub fn read(name: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SymbolicRefAction::Read {
                name: name.into(),
                short: false,
            },
        }
    }

    /// `--short` for a read (only applies when the action is [`read`](Self::read)).
    pub fn short(&mut self) -> &mut Self {
        if let SymbolicRefAction::Read { short, .. } = &mut self.action {
            *short = true;
        }
        self
    }

    /// Set `name` to point at `target`.
    pub fn set(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SymbolicRefAction::Set {
                name: name.into(),
                target: target.into(),
                reason: None,
            },
        }
    }

    /// Set the reflog reason (`-m`, only for [`set`](Self::set)).
    pub fn reason(&mut self, r: impl Into<String>) -> &mut Self {
        if let SymbolicRefAction::Set { reason, .. } = &mut self.action {
            *reason = Some(r.into());
        }
        self
    }

    /// Delete the symbolic ref.
    pub fn delete(name: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SymbolicRefAction::Delete {
                name: name.into(),
                quiet: false,
            },
        }
    }

    /// `-q` (only for [`delete`](Self::delete)).
    pub fn quiet(&mut self) -> &mut Self {
        if let SymbolicRefAction::Delete { quiet, .. } = &mut self.action {
            *quiet = true;
        }
        self
    }
}

#[async_trait]
impl GitCommand for SymbolicRefCommand {
    /// Trimmed stdout — the resolved target for `read`, empty for `set` / `delete`.
    type Output = String;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["symbolic-ref".to_string()];
        match &self.action {
            SymbolicRefAction::Read { name, short } => {
                if *short {
                    args.push("--short".into());
                }
                args.push(name.clone());
            }
            SymbolicRefAction::Set {
                name,
                target,
                reason,
            } => {
                if let Some(r) = reason {
                    args.push("-m".into());
                    args.push(r.clone());
                }
                args.push(name.clone());
                args.push(target.clone());
            }
            SymbolicRefAction::Delete { name, quiet } => {
                args.push("--delete".into());
                if *quiet {
                    args.push("-q".into());
                }
                args.push(name.clone());
            }
        }
        args
    }

    async fn execute(&self) -> Result<String> {
        if let SymbolicRefAction::Read { name, .. } | SymbolicRefAction::Set { name, .. } =
            &self.action
        {
            if name.is_empty() {
                return Err(Error::invalid_config("symbolic-ref requires a ref name"));
            }
        }
        let out = self.execute_raw().await?;
        Ok(out.stdout_trimmed())
    }
}
