//! `git update-ref` — update the object name stored in a ref safely.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git update-ref`.
#[derive(Debug, Clone, Default)]
pub struct UpdateRefCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Ref name (e.g. `"refs/heads/main"`).
    pub ref_name: Option<String>,
    /// New object.
    pub new_value: Option<String>,
    /// Expected old object (for safe update).
    pub old_value: Option<String>,
    /// `-d` delete mode.
    pub delete: bool,
    /// `--no-deref`.
    pub no_deref: bool,
    /// `-m <reason>`.
    pub message: Option<String>,
}

impl UpdateRefCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set ref name.
    pub fn ref_name(&mut self, r: impl Into<String>) -> &mut Self {
        self.ref_name = Some(r.into());
        self
    }

    /// Set new value.
    pub fn new_value(&mut self, v: impl Into<String>) -> &mut Self {
        self.new_value = Some(v.into());
        self
    }

    /// Set expected old value (compare-and-set).
    pub fn old_value(&mut self, v: impl Into<String>) -> &mut Self {
        self.old_value = Some(v.into());
        self
    }

    /// Delete the ref.
    pub fn delete(&mut self) -> &mut Self {
        self.delete = true;
        self
    }

    /// `--no-deref`.
    pub fn no_deref(&mut self) -> &mut Self {
        self.no_deref = true;
        self
    }

    /// Reflog message.
    pub fn message(&mut self, m: impl Into<String>) -> &mut Self {
        self.message = Some(m.into());
        self
    }
}

#[async_trait]
impl GitCommand for UpdateRefCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["update-ref".to_string()];
        if self.no_deref {
            args.push("--no-deref".into());
        }
        if let Some(m) = &self.message {
            args.push("-m".into());
            args.push(m.clone());
        }
        if self.delete {
            args.push("-d".into());
            if let Some(r) = &self.ref_name {
                args.push(r.clone());
            }
            if let Some(o) = &self.old_value {
                args.push(o.clone());
            }
            return args;
        }
        if let Some(r) = &self.ref_name {
            args.push(r.clone());
        }
        if let Some(v) = &self.new_value {
            args.push(v.clone());
        }
        if let Some(o) = &self.old_value {
            args.push(o.clone());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        if self.ref_name.is_none() {
            return Err(Error::invalid_config("update-ref requires a ref name"));
        }
        if !self.delete && self.new_value.is_none() {
            return Err(Error::invalid_config(
                "update-ref requires a new value unless --delete is set",
            ));
        }
        self.execute_raw().await
    }
}
