//! `git merge-base` — find common ancestors of two or more commits.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git merge-base`.
///
/// Reports the best common ancestor of the given commits, one SHA per line.
/// Three shapes are covered, selected by the mode flags:
///
/// - the default, `<commit> <commit>...` — the best common ancestor, or every
///   one of them with [`all`](Self::all);
/// - [`is_ancestor`](Self::is_ancestor), `<commit> <commit>` — no output, the
///   answer is the exit code;
/// - [`fork_point`](Self::fork_point), `<ref> [<commit>]` — where the commit
///   forked from the ref, using the ref's reflog.
///
/// Each shape takes a different number of positional commits, so
/// [`execute`](GitCommand::execute) checks the count against the selected mode
/// rather than letting git fail on a usage error. `--is-ancestor` combines with
/// neither of the other two, and that is rejected the same way.
///
/// # Exit codes carry answers
///
/// git exits 1 when there is no merge base at all (unrelated histories, or a
/// `--fork-point` it could not find), and `--is-ancestor` exits 1 for "no".
/// [`execute`](GitCommand::execute) surfaces both as [`Error::CommandFailed`],
/// which is indistinguishable from a real failure without inspecting the exit
/// code, so two methods decode them instead:
/// [`execute_is_ancestor`](Self::execute_is_ancestor) returns the `--is-ancestor`
/// answer as a `bool`, and
/// [`execute_allow_no_base`](Self::execute_allow_no_base) returns `Ok(None)`
/// for the other modes when no base exists.
///
/// `--octopus` and `--independent` are left to the documented raw-argument
/// escape hatch.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MergeBaseCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The positional commit arguments, in the order they were added.
    pub commits: Vec<String>,
    /// `--all`: output every best common ancestor instead of just one.
    pub all: bool,
    /// `--is-ancestor`: report whether the first commit is an ancestor of the
    /// second through the exit code.
    pub is_ancestor: bool,
    /// `--fork-point`: find where the commit forked from the ref, using the
    /// ref's reflog rather than reachability alone.
    pub fork_point: bool,
}

impl MergeBaseCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one positional commit argument.
    pub fn commit(&mut self, commit: impl Into<String>) -> &mut Self {
        self.commits.push(commit.into());
        self
    }

    /// Append several positional commit arguments.
    pub fn commits<I, S>(&mut self, commits: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.commits.extend(commits.into_iter().map(Into::into));
        self
    }

    /// Output every best common ancestor, not just one.
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }

    /// Ask whether the first commit is an ancestor of the second.
    ///
    /// The answer is the exit code and there is no output, so read it with
    /// [`execute_is_ancestor`](Self::execute_is_ancestor).
    pub fn is_ancestor(&mut self) -> &mut Self {
        self.is_ancestor = true;
        self
    }

    /// Find where the commit forked from the ref's reflog.
    pub fn fork_point(&mut self) -> &mut Self {
        self.fork_point = true;
        self
    }

    /// Run `--is-ancestor` and return the answer.
    ///
    /// `Ok(true)` when the first commit is an ancestor of the second,
    /// `Ok(false)` when it is not. Genuine failures (a bad revision, exit 128)
    /// still return `Err`. Requires [`is_ancestor`](Self::is_ancestor); without
    /// it the command has different output and exit-code meanings, so this
    /// returns [`Error::InvalidConfig`].
    pub async fn execute_is_ancestor(&self) -> Result<bool> {
        if !self.is_ancestor {
            return Err(Error::invalid_config(
                "merge-base: execute_is_ancestor requires --is-ancestor",
            ));
        }
        self.validate()?;
        match self.execute_raw().await {
            Ok(_) => Ok(true),
            Err(Error::CommandFailed { exit_code: 1, .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Run the command, treating "no merge base" as success instead of an error.
    ///
    /// Returns `Ok(None)` when git exits 1 because the commits share no history
    /// or the fork point could not be found, and `Ok(Some(output))` otherwise.
    /// Rejects [`is_ancestor`](Self::is_ancestor), where exit 1 means "not an
    /// ancestor" rather than "no base": use
    /// [`execute_is_ancestor`](Self::execute_is_ancestor) for that mode.
    pub async fn execute_allow_no_base(&self) -> Result<Option<CommandOutput>> {
        if self.is_ancestor {
            return Err(Error::invalid_config(
                "merge-base: --is-ancestor exits 1 for \"not an ancestor\", use execute_is_ancestor",
            ));
        }
        self.validate()?;
        match self.execute_raw().await {
            Ok(out) => Ok(Some(out)),
            Err(Error::CommandFailed { exit_code: 1, .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Check the mode flags against each other and against the commit count.
    fn validate(&self) -> Result<()> {
        if self.is_ancestor && self.fork_point {
            return Err(Error::invalid_config(
                "merge-base: --is-ancestor and --fork-point cannot be used together",
            ));
        }
        if self.is_ancestor && self.all {
            return Err(Error::invalid_config(
                "merge-base: --is-ancestor and --all cannot be used together",
            ));
        }

        let n = self.commits.len();
        if self.is_ancestor {
            if n != 2 {
                return Err(Error::invalid_config(format!(
                    "merge-base: --is-ancestor takes exactly two commits, got {n}"
                )));
            }
        } else if self.fork_point {
            if n == 0 || n > 2 {
                return Err(Error::invalid_config(format!(
                    "merge-base: --fork-point takes a ref and an optional commit, got {n} arguments"
                )));
            }
        } else if n < 2 {
            return Err(Error::invalid_config(format!(
                "merge-base requires at least two commits, got {n}"
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl GitCommand for MergeBaseCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["merge-base".to_string()];
        if self.all {
            args.push("--all".into());
        }
        if self.is_ancestor {
            args.push("--is-ancestor".into());
        }
        if self.fork_point {
            args.push("--fork-point".into());
        }
        args.extend(self.commits.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.validate()?;
        self.execute_raw().await
    }
}
