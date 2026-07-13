//! `git cherry-pick` — apply the changes introduced by some existing commits.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git cherry-pick`.
#[derive(Debug, Clone, Default)]
pub struct CherryPickCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Commits (or ranges) to pick.
    pub commits: Vec<String>,
    /// `--no-commit` / `-n`.
    pub no_commit: bool,
    /// `--edit`.
    pub edit: bool,
    /// `--signoff` / `-s`.
    pub signoff: bool,
    /// `-x` append "cherry picked from commit …".
    pub reference: bool,
    /// `--mainline N` (for merges).
    pub mainline: Option<u32>,
    /// `--strategy`.
    pub strategy: Option<String>,
    /// `--abort`.
    pub abort: bool,
    /// `--continue`.
    pub cont: bool,
    /// `--skip`.
    pub skip: bool,
    /// `--quit`.
    pub quit: bool,
    /// `--allow-empty`.
    pub allow_empty: bool,
    /// `--keep-redundant-commits`.
    pub keep_redundant: bool,
}

impl CherryPickCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a commit / range.
    pub fn commit(&mut self, c: impl Into<String>) -> &mut Self {
        self.commits.push(c.into());
        self
    }

    /// Do not commit automatically.
    pub fn no_commit(&mut self) -> &mut Self {
        self.no_commit = true;
        self
    }

    /// Open the editor for the commit message.
    pub fn edit(&mut self) -> &mut Self {
        self.edit = true;
        self
    }

    /// Add `Signed-off-by`.
    pub fn signoff(&mut self) -> &mut Self {
        self.signoff = true;
        self
    }

    /// Append `(cherry picked from commit …)` to the message.
    pub fn reference(&mut self) -> &mut Self {
        self.reference = true;
        self
    }

    /// For merge commits, specify the mainline parent.
    pub fn mainline(&mut self, n: u32) -> &mut Self {
        self.mainline = Some(n);
        self
    }

    /// Merge strategy.
    pub fn strategy(&mut self, s: impl Into<String>) -> &mut Self {
        self.strategy = Some(s.into());
        self
    }

    /// Abort an in-progress cherry-pick.
    pub fn abort(&mut self) -> &mut Self {
        self.abort = true;
        self
    }

    /// Continue after resolving conflicts.
    pub fn cont(&mut self) -> &mut Self {
        self.cont = true;
        self
    }

    /// Skip the current commit.
    pub fn skip(&mut self) -> &mut Self {
        self.skip = true;
        self
    }

    /// Forget the in-progress cherry-pick state.
    pub fn quit(&mut self) -> &mut Self {
        self.quit = true;
        self
    }

    /// Allow empty commits.
    pub fn allow_empty(&mut self) -> &mut Self {
        self.allow_empty = true;
        self
    }

    /// Keep commits that become empty due to the pick.
    pub fn keep_redundant(&mut self) -> &mut Self {
        self.keep_redundant = true;
        self
    }

    /// Classify a cherry-pick's [`CommandOutput`] into a
    /// [`CherryPickResult`](crate::parse::CherryPickResult).
    ///
    /// Returns `None` for `--abort` / `--continue` / `--skip` / `--quit`
    /// invocations, since their output does not describe a pick outcome.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_result(&self, output: &CommandOutput) -> Option<crate::parse::CherryPickResult> {
        if self.abort || self.cont || self.skip || self.quit {
            return None;
        }
        Some(crate::parse::parse_cherry_pick(output))
    }
}

#[async_trait]
impl GitCommand for CherryPickCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["cherry-pick".to_string()];
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
        if self.no_commit {
            args.push("--no-commit".into());
        }
        if self.edit {
            args.push("--edit".into());
        }
        if self.signoff {
            args.push("--signoff".into());
        }
        if self.reference {
            args.push("-x".into());
        }
        if self.allow_empty {
            args.push("--allow-empty".into());
        }
        if self.keep_redundant {
            args.push("--keep-redundant-commits".into());
        }
        if let Some(m) = self.mainline {
            args.push("--mainline".into());
            args.push(m.to_string());
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
    fn parse_result_clean_pick() {
        let c = CherryPickCommand::new();
        let result = c
            .parse_result(&output("[main abc1234] add feature\n"))
            .unwrap();
        assert!(!result.conflicts);
    }

    #[test]
    fn parse_result_conflict() {
        let c = CherryPickCommand::new();
        let result = c
            .parse_result(&output("CONFLICT (content): Merge conflict in a.txt\n"))
            .unwrap();
        assert!(result.conflicts);
    }

    #[test]
    fn parse_result_none_for_abort() {
        let mut c = CherryPickCommand::new();
        c.abort();
        assert!(c.parse_result(&output("")).is_none());
    }

    #[test]
    fn parse_result_none_for_continue() {
        let mut c = CherryPickCommand::new();
        c.cont();
        assert!(c.parse_result(&output("")).is_none());
    }

    #[test]
    fn parse_result_none_for_skip() {
        let mut c = CherryPickCommand::new();
        c.skip();
        assert!(c.parse_result(&output("")).is_none());
    }

    #[test]
    fn parse_result_none_for_quit() {
        let mut c = CherryPickCommand::new();
        c.quit();
        assert!(c.parse_result(&output("")).is_none());
    }
}
