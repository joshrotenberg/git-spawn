//! `git merge` — join two or more development histories together.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git merge`.
#[derive(Debug, Clone, Default)]
pub struct MergeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Commits/branches to merge.
    pub commits: Vec<String>,
    /// `--no-ff`.
    pub no_ff: bool,
    /// `--ff-only`.
    pub ff_only: bool,
    /// `--squash`.
    pub squash: bool,
    /// `--commit`.
    pub commit: bool,
    /// `--no-commit`.
    pub no_commit: bool,
    /// `-m`.
    pub message: Option<String>,
    /// `--strategy`.
    pub strategy: Option<String>,
    /// `--abort`.
    pub abort: bool,
    /// `--continue`.
    pub cont: bool,
    /// `--quiet`.
    pub quiet: bool,
}

impl MergeCommand {
    /// New `merge`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a commit to merge.
    pub fn commit_ref(&mut self, c: impl Into<String>) -> &mut Self {
        self.commits.push(c.into());
        self
    }

    /// `--no-ff`.
    pub fn no_ff(&mut self) -> &mut Self {
        self.no_ff = true;
        self
    }

    /// `--ff-only`.
    pub fn ff_only(&mut self) -> &mut Self {
        self.ff_only = true;
        self
    }

    /// `--squash`.
    pub fn squash(&mut self) -> &mut Self {
        self.squash = true;
        self
    }

    /// Always create a commit.
    pub fn commit(&mut self) -> &mut Self {
        self.commit = true;
        self
    }

    /// Don't commit.
    pub fn no_commit(&mut self) -> &mut Self {
        self.no_commit = true;
        self
    }

    /// Merge message.
    pub fn message(&mut self, m: impl Into<String>) -> &mut Self {
        self.message = Some(m.into());
        self
    }

    /// Merge strategy.
    pub fn strategy(&mut self, s: impl Into<String>) -> &mut Self {
        self.strategy = Some(s.into());
        self
    }

    /// Abort an in-progress merge.
    pub fn abort(&mut self) -> &mut Self {
        self.abort = true;
        self
    }

    /// Continue an in-progress merge.
    pub fn cont(&mut self) -> &mut Self {
        self.cont = true;
        self
    }

    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }

    /// Classify a completed merge's [`CommandOutput`] into a
    /// [`MergeResult`](crate::parse::MergeResult).
    ///
    /// Returns `None` for `--abort` / `--continue` invocations, since their
    /// output matches neither the fast-forward nor the already-up-to-date
    /// substring and does not describe a merge outcome.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_result(&self, output: &CommandOutput) -> Option<crate::parse::MergeResult> {
        if self.abort || self.cont {
            return None;
        }
        Some(crate::parse::parse_merge(&output.stdout_str()))
    }
}

#[async_trait]
impl GitCommand for MergeCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["merge".to_string()];
        if self.abort {
            args.push("--abort".into());
            return args;
        }
        if self.cont {
            args.push("--continue".into());
            return args;
        }
        if self.no_ff {
            args.push("--no-ff".into());
        }
        if self.ff_only {
            args.push("--ff-only".into());
        }
        if self.squash {
            args.push("--squash".into());
        }
        if self.commit {
            args.push("--commit".into());
        }
        if self.no_commit {
            args.push("--no-commit".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if let Some(m) = &self.message {
            args.push("-m".into());
            args.push(m.clone());
        }
        if let Some(s) = &self.strategy {
            args.push(format!("--strategy={s}"));
        }
        args.extend(self.commits.iter().cloned());
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}

#[cfg(all(test, feature = "parse"))]
mod tests {
    use super::*;

    fn output(stdout: &str) -> CommandOutput {
        CommandOutput {
            stdout: stdout.as_bytes().to_vec(),
            stderr: String::new(),
            exit_code: 0,
            success: true,
        }
    }

    #[test]
    fn parse_result_fast_forward() {
        let c = MergeCommand::new();
        let result = c
            .parse_result(&output("Updating a1b2c3..d4e5f6\nFast-forward\n"))
            .unwrap();
        assert!(result.fast_forward);
        assert!(!result.already_up_to_date);
    }

    #[test]
    fn parse_result_already_up_to_date() {
        let c = MergeCommand::new();
        let result = c.parse_result(&output("Already up to date.\n")).unwrap();
        assert!(!result.fast_forward);
        assert!(result.already_up_to_date);
    }

    #[test]
    fn parse_result_none_for_abort() {
        let mut c = MergeCommand::new();
        c.abort();
        assert!(c.parse_result(&output("")).is_none());
    }

    #[test]
    fn parse_result_none_for_continue() {
        let mut c = MergeCommand::new();
        c.cont();
        assert!(c.parse_result(&output("")).is_none());
    }
}
