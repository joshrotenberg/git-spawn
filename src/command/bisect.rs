//! `git bisect` — find the commit that introduced a bug via binary search.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Actions supported by `git bisect`.
#[derive(Debug, Clone)]
pub enum BisectAction {
    /// `git bisect start [<bad>] [<good>…]`.
    Start {
        /// Initial known-bad commit.
        bad: Option<String>,
        /// Known-good commits.
        good: Vec<String>,
    },
    /// `git bisect good [<rev>…]`.
    Good(Vec<String>),
    /// `git bisect bad [<rev>]`.
    Bad(Option<String>),
    /// `git bisect skip [<rev>…]`.
    Skip(Vec<String>),
    /// `git bisect reset [<commit>]`.
    Reset(Option<String>),
    /// `git bisect log`.
    Log,
    /// `git bisect replay <log>`.
    Replay(PathBuf),
    /// `git bisect run <cmd> [<args>…]`.
    Run(Vec<String>),
}

/// Builder for `git bisect`.
#[derive(Debug, Clone)]
pub struct BisectCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: BisectAction,
}

impl BisectCommand {
    /// `bisect start`.
    #[must_use]
    pub fn start() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Start {
                bad: None,
                good: vec![],
            },
        }
    }

    /// Set initial `bad` commit.
    pub fn bad_commit(&mut self, c: impl Into<String>) -> &mut Self {
        if let BisectAction::Start { bad, .. } = &mut self.action {
            *bad = Some(c.into());
        }
        self
    }

    /// Add a known-`good` commit (for `start`).
    pub fn good_commit(&mut self, c: impl Into<String>) -> &mut Self {
        if let BisectAction::Start { good, .. } = &mut self.action {
            good.push(c.into());
        }
        self
    }

    /// `bisect good` with optional revs.
    #[must_use]
    pub fn good(revs: Vec<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Good(revs),
        }
    }

    /// `bisect bad` with optional rev.
    #[must_use]
    pub fn bad(rev: Option<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Bad(rev),
        }
    }

    /// `bisect skip`.
    #[must_use]
    pub fn skip(revs: Vec<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Skip(revs),
        }
    }

    /// `bisect reset`.
    #[must_use]
    pub fn reset(commit: Option<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Reset(commit),
        }
    }

    /// `bisect log`.
    #[must_use]
    pub fn log() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Log,
        }
    }

    /// `bisect replay <log>`.
    pub fn replay(path: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Replay(path.into()),
        }
    }

    /// `bisect run <cmd> [args…]`.
    pub fn run<I, S>(command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            executor: CommandExecutor::default(),
            action: BisectAction::Run(command.into_iter().map(Into::into).collect()),
        }
    }
}

#[async_trait]
impl GitCommand for BisectCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["bisect".to_string()];
        match &self.action {
            BisectAction::Start { bad, good } => {
                args.push("start".into());
                if let Some(b) = bad {
                    args.push(b.clone());
                }
                args.extend(good.iter().cloned());
            }
            BisectAction::Good(revs) => {
                args.push("good".into());
                args.extend(revs.iter().cloned());
            }
            BisectAction::Bad(rev) => {
                args.push("bad".into());
                if let Some(r) = rev {
                    args.push(r.clone());
                }
            }
            BisectAction::Skip(revs) => {
                args.push("skip".into());
                args.extend(revs.iter().cloned());
            }
            BisectAction::Reset(c) => {
                args.push("reset".into());
                if let Some(c) = c {
                    args.push(c.clone());
                }
            }
            BisectAction::Log => args.push("log".into()),
            BisectAction::Replay(p) => {
                args.push("replay".into());
                args.push(p.display().to_string());
            }
            BisectAction::Run(cmd) => {
                args.push("run".into());
                args.extend(cmd.iter().cloned());
            }
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
