//! `git revert` — undo commits by recording new commits that reverse them.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git revert`.
///
/// Reverting records a new commit per named commit-ish, so a revert of a
/// merge needs [`mainline`](Self::mainline) to say which parent's line of
/// history to treat as the mainline.
///
/// Unlike `git cherry-pick`, `git revert` opens an editor for the generated
/// commit message by default. There is no terminal behind
/// [`CommandExecutor`], so call [`no_edit`](Self::no_edit) to take git's
/// generated message as-is; otherwise git invokes the configured editor and
/// fails when it cannot run one.
///
/// The session controls [`cont`](Self::cont), [`skip`](Self::skip) and
/// [`abort`](Self::abort) act on a revert that stopped on a conflict. Each
/// takes no other arguments, so setting one makes the command a bare
/// `git revert --<action>`.
#[derive(Debug, Clone, Default)]
pub struct RevertCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Commits (or ranges) to revert.
    pub commits: Vec<String>,
    /// `--no-commit` / `-n`: stage the reversal without committing.
    pub no_commit: bool,
    /// `--mainline <n>`: the parent number for reverting a merge.
    pub mainline: Option<u32>,
    /// `--no-edit`: keep the generated message without opening an editor.
    pub no_edit: bool,
    /// `--continue`.
    pub cont: bool,
    /// `--skip`.
    pub skip: bool,
    /// `--abort`.
    pub abort: bool,
}

impl RevertCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Revert `commit`. Repeatable; each call adds one positional argument.
    pub fn commit(&mut self, commit: impl Into<String>) -> &mut Self {
        self.commits.push(commit.into());
        self
    }

    /// Stage the reversal in the index and working tree without committing.
    pub fn no_commit(&mut self) -> &mut Self {
        self.no_commit = true;
        self
    }

    /// Treat parent number `n` as the mainline when reverting a merge.
    pub fn mainline(&mut self, n: u32) -> &mut Self {
        self.mainline = Some(n);
        self
    }

    /// Accept the generated commit message instead of opening an editor.
    pub fn no_edit(&mut self) -> &mut Self {
        self.no_edit = true;
        self
    }

    /// Continue a revert that stopped on a conflict.
    pub fn cont(&mut self) -> &mut Self {
        self.cont = true;
        self
    }

    /// Skip the commit that a stopped revert is on.
    pub fn skip(&mut self) -> &mut Self {
        self.skip = true;
        self
    }

    /// Abort a stopped revert and restore the pre-revert state.
    pub fn abort(&mut self) -> &mut Self {
        self.abort = true;
        self
    }

    /// Whether a session control is set.
    fn is_session_action(&self) -> bool {
        self.cont || self.skip || self.abort
    }
}

#[async_trait]
impl GitCommand for RevertCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["revert".to_string()];
        if self.cont {
            args.push("--continue".into());
            return args;
        }
        if self.skip {
            args.push("--skip".into());
            return args;
        }
        if self.abort {
            args.push("--abort".into());
            return args;
        }
        if self.no_commit {
            args.push("--no-commit".into());
        }
        if self.no_edit {
            args.push("--no-edit".into());
        }
        if let Some(n) = self.mainline {
            args.push("--mainline".into());
            args.push(n.to_string());
        }
        args.extend(self.commits.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.is_session_action() {
            if [self.cont, self.skip, self.abort]
                .iter()
                .filter(|set| **set)
                .count()
                > 1
            {
                return Err(Error::invalid_config(
                    "revert: --continue, --skip and --abort are mutually exclusive",
                ));
            }
        } else if self.commits.is_empty() {
            return Err(Error::invalid_config(
                "revert: no commit given, and no session action to continue",
            ));
        }
        self.execute_raw().await
    }
}
