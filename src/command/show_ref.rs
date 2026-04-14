//! `git show-ref` — list references in a local repository.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git show-ref`.
#[derive(Debug, Clone, Default)]
pub struct ShowRefCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Patterns to match.
    pub patterns: Vec<String>,
    /// `--heads`.
    pub heads: bool,
    /// `--tags`.
    pub tags: bool,
    /// `--verify`: error if a pattern doesn't match.
    pub verify: bool,
    /// `--hash[=N]`.
    pub hash: Option<Option<u32>>,
    /// `--dereference`.
    pub dereference: bool,
    /// `--head`.
    pub head: bool,
    /// `--exists`: test existence of a specific ref.
    pub exists: bool,
    /// `-q` quiet.
    pub quiet: bool,
}

impl ShowRefCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a ref pattern.
    pub fn pattern(&mut self, p: impl Into<String>) -> &mut Self {
        self.patterns.push(p.into());
        self
    }

    /// Limit to branch refs.
    pub fn heads(&mut self) -> &mut Self {
        self.heads = true;
        self
    }

    /// Limit to tag refs.
    pub fn tags(&mut self) -> &mut Self {
        self.tags = true;
        self
    }

    /// `--verify`.
    pub fn verify(&mut self) -> &mut Self {
        self.verify = true;
        self
    }

    /// `--hash`.
    pub fn hash(&mut self) -> &mut Self {
        self.hash = Some(None);
        self
    }

    /// `--hash=N`.
    pub fn hash_len(&mut self, n: u32) -> &mut Self {
        self.hash = Some(Some(n));
        self
    }

    /// `--dereference`.
    pub fn dereference(&mut self) -> &mut Self {
        self.dereference = true;
        self
    }

    /// Include HEAD in output (`--head`).
    pub fn include_head(&mut self) -> &mut Self {
        self.head = true;
        self
    }

    /// `--exists`.
    pub fn exists(&mut self) -> &mut Self {
        self.exists = true;
        self
    }

    /// `-q`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for ShowRefCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["show-ref".to_string()];
        if self.heads {
            args.push("--heads".into());
        }
        if self.tags {
            args.push("--tags".into());
        }
        if self.head {
            args.push("--head".into());
        }
        if self.dereference {
            args.push("--dereference".into());
        }
        if self.verify {
            args.push("--verify".into());
        }
        if self.exists {
            args.push("--exists".into());
        }
        if self.quiet {
            args.push("-q".into());
        }
        match self.hash {
            Some(None) => args.push("--hash".into()),
            Some(Some(n)) => args.push(format!("--hash={n}")),
            None => {}
        }
        args.extend(self.patterns.iter().cloned());
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
