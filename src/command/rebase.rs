//! `git rebase` — reapply commits on top of another base tip.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git rebase`.
#[derive(Debug, Clone, Default)]
pub struct RebaseCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Upstream base.
    pub upstream: Option<String>,
    /// Branch to rebase.
    pub branch: Option<String>,
    /// `--onto`.
    pub onto: Option<String>,
    /// `--interactive` / `-i`.
    pub interactive: bool,
    /// `--autosquash`.
    pub autosquash: bool,
    /// `--autostash`.
    pub autostash: bool,
    /// `--abort`.
    pub abort: bool,
    /// `--continue`.
    pub cont: bool,
    /// `--skip`.
    pub skip: bool,
    /// `--quit`.
    pub quit: bool,
    /// `--root`.
    pub root: bool,
    /// `--strategy`.
    pub strategy: Option<String>,
}

impl RebaseCommand {
    /// New `rebase`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream base.
    pub fn upstream(&mut self, u: impl Into<String>) -> &mut Self {
        self.upstream = Some(u.into());
        self
    }

    /// Branch to rebase.
    pub fn branch(&mut self, b: impl Into<String>) -> &mut Self {
        self.branch = Some(b.into());
        self
    }

    /// `--onto`.
    pub fn onto(&mut self, o: impl Into<String>) -> &mut Self {
        self.onto = Some(o.into());
        self
    }

    /// Interactive mode.
    pub fn interactive(&mut self) -> &mut Self {
        self.interactive = true;
        self
    }

    /// `--autosquash`.
    pub fn autosquash(&mut self) -> &mut Self {
        self.autosquash = true;
        self
    }

    /// `--autostash`.
    pub fn autostash(&mut self) -> &mut Self {
        self.autostash = true;
        self
    }

    /// `--abort`.
    pub fn abort(&mut self) -> &mut Self {
        self.abort = true;
        self
    }

    /// `--continue`.
    pub fn cont(&mut self) -> &mut Self {
        self.cont = true;
        self
    }

    /// `--skip`.
    pub fn skip(&mut self) -> &mut Self {
        self.skip = true;
        self
    }

    /// `--quit`.
    pub fn quit(&mut self) -> &mut Self {
        self.quit = true;
        self
    }

    /// `--root`.
    pub fn root(&mut self) -> &mut Self {
        self.root = true;
        self
    }

    /// Merge strategy.
    pub fn strategy(&mut self, s: impl Into<String>) -> &mut Self {
        self.strategy = Some(s.into());
        self
    }
}

#[async_trait]
impl GitCommand for RebaseCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["rebase".to_string()];
        if self.abort {
            args.push("--abort".into());
            return args;
        }
        if self.cont {
            args.push("--continue".into());
            return args;
        }
        if self.skip {
            args.push("--skip".into());
            return args;
        }
        if self.quit {
            args.push("--quit".into());
            return args;
        }
        if self.interactive {
            args.push("--interactive".into());
        }
        if self.autosquash {
            args.push("--autosquash".into());
        }
        if self.autostash {
            args.push("--autostash".into());
        }
        if self.root {
            args.push("--root".into());
        }
        if let Some(o) = &self.onto {
            args.push("--onto".into());
            args.push(o.clone());
        }
        if let Some(s) = &self.strategy {
            args.push(format!("--strategy={s}"));
        }
        if let Some(u) = &self.upstream {
            args.push(u.clone());
        }
        if let Some(b) = &self.branch {
            args.push(b.clone());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
