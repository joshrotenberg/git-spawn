//! `git range-diff` — compare two commit ranges.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git range-diff`.
///
/// Compares two versions of a patch series and reports, commit by commit, which
/// patches are unchanged, changed, added or dropped. It is the tool for
/// reviewing a rebase or a force-push: `range-diff` pairs commits up by content
/// rather than by position.
///
/// git accepts the revision arguments in three shapes, distinguished only by
/// how many are given:
///
/// - one argument, `<rev1>...<rev2>` — the symmetric difference form;
/// - two arguments, `<range1> <range2>`;
/// - three arguments, `<base> <rev1> <rev2>`.
///
/// [`rev`](Self::rev) appends one positional argument per call, so the shape is
/// chosen by how many times it is called. Zero arguments, or more than three,
/// is rejected by [`execute`](GitCommand::execute) rather than handed to git.
///
/// Output is left as a [`CommandOutput`]: the report is a diff of diffs, and
/// this crate does not model it as typed values.
#[derive(Debug, Clone, Default)]
pub struct RangeDiffCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The positional revision arguments, in the order they were added.
    pub revs: Vec<String>,
    /// `--creation-factor=<n>`: percentage by which creation of a new commit is
    /// weighted when pairing commits up. Higher values pair more aggressively.
    pub creation_factor: Option<u32>,
    /// `--no-dual-color`: colour the output as a plain diff instead of using
    /// git's two-tone scheme.
    pub no_dual_color: bool,
    /// `--left-only`: only report commits from the first range.
    pub left_only: bool,
    /// `--right-only`: only report commits from the second range.
    pub right_only: bool,
}

impl RangeDiffCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one positional revision argument.
    ///
    /// Call it once for the `<rev1>...<rev2>` form, twice for
    /// `<range1> <range2>`, or three times for `<base> <rev1> <rev2>`.
    pub fn rev(&mut self, rev: impl Into<String>) -> &mut Self {
        self.revs.push(rev.into());
        self
    }

    /// Weight commit creation by `percent` when pairing commits up.
    pub fn creation_factor(&mut self, percent: u32) -> &mut Self {
        self.creation_factor = Some(percent);
        self
    }

    /// Colour the output as a plain diff.
    pub fn no_dual_color(&mut self) -> &mut Self {
        self.no_dual_color = true;
        self
    }

    /// Only report commits from the first range.
    pub fn left_only(&mut self) -> &mut Self {
        self.left_only = true;
        self
    }

    /// Only report commits from the second range.
    pub fn right_only(&mut self) -> &mut Self {
        self.right_only = true;
        self
    }
}

#[async_trait]
impl GitCommand for RangeDiffCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["range-diff".to_string()];
        if self.no_dual_color {
            args.push("--no-dual-color".into());
        }
        if let Some(percent) = self.creation_factor {
            args.push(format!("--creation-factor={percent}"));
        }
        if self.left_only {
            args.push("--left-only".into());
        }
        if self.right_only {
            args.push("--right-only".into());
        }
        args.extend(self.revs.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        match self.revs.len() {
            1..=3 => self.execute_raw().await,
            0 => Err(Error::invalid_config(
                "range-diff requires one, two or three revision arguments",
            )),
            n => Err(Error::invalid_config(format!(
                "range-diff accepts at most three revision arguments, got {n}"
            ))),
        }
    }
}
