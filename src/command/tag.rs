//! `git tag` — create, list, delete, or verify tags.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git tag`.
#[derive(Debug, Clone, Default)]
pub struct TagCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Tag name.
    pub name: Option<String>,
    /// Commit-ish to tag.
    pub commit: Option<String>,
    /// `-a` annotated.
    pub annotated: bool,
    /// `-m` message.
    pub message: Option<String>,
    /// `-s` signed.
    pub signed: bool,
    /// `-f` force.
    pub force: bool,
    /// `-d` delete.
    pub delete: bool,
    /// `-l` list mode.
    pub list: bool,
    /// Pattern filter (for list).
    pub pattern: Option<String>,
}

impl TagCommand {
    /// New `tag` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Name.
    pub fn name(&mut self, n: impl Into<String>) -> &mut Self {
        self.name = Some(n.into());
        self
    }

    /// Commit to tag.
    pub fn commit(&mut self, c: impl Into<String>) -> &mut Self {
        self.commit = Some(c.into());
        self
    }

    /// Annotated.
    pub fn annotated(&mut self) -> &mut Self {
        self.annotated = true;
        self
    }

    /// Tag message.
    pub fn message(&mut self, m: impl Into<String>) -> &mut Self {
        self.message = Some(m.into());
        self.annotated = true;
        self
    }

    /// GPG-sign.
    pub fn signed(&mut self) -> &mut Self {
        self.signed = true;
        self
    }

    /// Force.
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }

    /// Delete.
    pub fn delete(&mut self) -> &mut Self {
        self.delete = true;
        self
    }

    /// List mode.
    pub fn list(&mut self) -> &mut Self {
        self.list = true;
        self
    }

    /// List pattern.
    pub fn pattern(&mut self, p: impl Into<String>) -> &mut Self {
        self.pattern = Some(p.into());
        self.list = true;
        self
    }
}

#[async_trait]
impl GitCommand for TagCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["tag".to_string()];
        if self.delete {
            args.push("-d".into());
            if let Some(n) = &self.name {
                args.push(n.clone());
            }
            return args;
        }
        if self.list {
            args.push("-l".into());
            if let Some(p) = &self.pattern {
                args.push(p.clone());
            }
            return args;
        }
        if self.annotated {
            args.push("-a".into());
        }
        if self.signed {
            args.push("-s".into());
        }
        if self.force {
            args.push("-f".into());
        }
        if let Some(m) = &self.message {
            args.push("-m".into());
            args.push(m.clone());
        }
        if let Some(n) = &self.name {
            args.push(n.clone());
        }
        if let Some(c) = &self.commit {
            args.push(c.clone());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
