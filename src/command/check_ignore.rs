//! `git check-ignore` — test paths against the repository's ignore rules.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git check-ignore`.
///
/// The command reports each input path that an exclude rule ignores. Git exits
/// with status 1 when no path matched; [`execute`](GitCommand::execute) keeps
/// the crate-wide non-zero-exit behavior, while
/// [`execute_allow_no_match`](Self::execute_allow_no_match) turns that expected
/// result into [`None`].
///
/// [`verbose`](Self::verbose) adds the matching rule's source, line number, and
/// pattern. [`non_matching`](Self::non_matching) includes unmatched paths in
/// that report and therefore requires verbose mode. [`no_index`](Self::no_index)
/// tests tracked paths too, instead of suppressing them merely because they are
/// already in the index.
///
/// Stdin and NUL-delimited modes are intentionally not modelled because the
/// executor does not pipe stdin. Paths are passed after `--`, so names that
/// begin with a dash cannot be mistaken for options.
#[derive(Debug, Clone, Default)]
pub struct CheckIgnoreCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Paths to test against the repository's ignore rules.
    pub paths: Vec<String>,
    /// `-v`: show the matching rule's source, line number, and pattern.
    pub verbose: bool,
    /// `-n`: include non-matching paths in verbose output.
    pub non_matching: bool,
    /// `--no-index`: do not suppress tracked paths.
    pub no_index: bool,
    /// `-q`: report the result through the exit status without output.
    pub quiet: bool,
}

impl CheckIgnoreCommand {
    /// New command with no paths yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Test `path` against the repository's ignore rules.
    pub fn path(&mut self, path: impl Into<String>) -> &mut Self {
        self.paths.push(path.into());
        self
    }

    /// Test several paths, preserving their input order.
    pub fn paths<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.paths.extend(paths.into_iter().map(Into::into));
        self
    }

    /// Show which exclude rule matched each reported path.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }

    /// Include paths that did not match an exclude rule.
    ///
    /// Git only accepts this option together with [`verbose`](Self::verbose).
    pub fn non_matching(&mut self) -> &mut Self {
        self.non_matching = true;
        self
    }

    /// Test tracked paths as well as untracked paths.
    pub fn no_index(&mut self) -> &mut Self {
        self.no_index = true;
        self
    }

    /// Suppress output and communicate the answer through the exit status.
    ///
    /// Git only accepts quiet mode with exactly one path and without verbose
    /// mode.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }

    /// Run the check, returning [`None`] when no path is ignored.
    ///
    /// Git uses exit status 1 for the ordinary "no match" result. Other
    /// failures, including invalid repositories and malformed raw arguments,
    /// remain errors.
    pub async fn execute_allow_no_match(&self) -> Result<Option<CommandOutput>> {
        self.validate()?;
        match self.execute_raw().await {
            Ok(output) => Ok(Some(output)),
            Err(Error::CommandFailed { exit_code: 1, .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn validate(&self) -> Result<()> {
        if self.paths.is_empty() {
            return Err(Error::invalid_config(
                "check-ignore: at least one path is required",
            ));
        }
        if self.non_matching && !self.verbose {
            return Err(Error::invalid_config(
                "check-ignore: --non-matching requires --verbose",
            ));
        }
        if self.quiet && self.verbose {
            return Err(Error::invalid_config(
                "check-ignore: --quiet cannot be combined with --verbose",
            ));
        }
        if self.quiet && self.paths.len() != 1 {
            return Err(Error::invalid_config(
                "check-ignore: --quiet requires exactly one path",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl GitCommand for CheckIgnoreCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["check-ignore".to_string()];
        if self.verbose {
            args.push("-v".into());
        }
        if self.non_matching {
            args.push("-n".into());
        }
        if self.no_index {
            args.push("--no-index".into());
        }
        if self.quiet {
            args.push("-q".into());
        }
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.validate()?;
        self.execute_raw().await
    }
}
