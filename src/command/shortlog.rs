//! `git shortlog` — summarize commit history grouped by author.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git shortlog`.
///
/// At least one revision is required. Given no revision argument and a
/// non-terminal standard input, `git shortlog` summarizes a log read from
/// stdin instead of the repository; [`CommandExecutor`] does not model stdin,
/// so that run would consume the parent process's input. [`execute`](GitCommand::execute)
/// rejects an empty revision list as [`Error::InvalidConfig`] rather than
/// leaving the command pointed at whatever stdin happens to be.
///
/// The default output lists each author with a count and the subject of every
/// commit. [`summary`](Self::summary) drops the subjects, leaving one line per
/// author. [`parse_entries`](Self::parse_entries) reads either shape.
///
/// Output is left as a [`CommandOutput`].
#[derive(Debug, Clone, Default)]
pub struct ShortlogCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Revisions and revision ranges to summarize.
    pub revs: Vec<String>,
    /// Pathspecs limiting which commits are counted.
    pub paths: Vec<String>,
    /// `-s`: counts only, without commit subjects.
    pub summary: bool,
    /// `-n`: sort authors by commit count instead of alphabetically.
    pub numbered: bool,
    /// `-e`: show author emails alongside names.
    pub email: bool,
    /// `-c`: group by committer rather than author.
    pub committer: bool,
    /// `-w<width>`: wrap subject lines at this width.
    pub wrap: Option<usize>,
}

impl ShortlogCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Summarize `rev`, which may be a single revision or a range such as
    /// `v1.0..HEAD`. Repeatable.
    pub fn rev(&mut self, rev: impl Into<String>) -> &mut Self {
        self.revs.push(rev.into());
        self
    }

    /// Limit the summary to commits touching `path`. Repeatable.
    pub fn path(&mut self, path: impl Into<String>) -> &mut Self {
        self.paths.push(path.into());
        self
    }

    /// Limit the summary to commits touching any of `paths`.
    pub fn paths<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.paths.extend(paths.into_iter().map(Into::into));
        self
    }

    /// Report counts only, omitting commit subjects.
    pub fn summary(&mut self) -> &mut Self {
        self.summary = true;
        self
    }

    /// Sort authors by commit count, highest first.
    pub fn numbered(&mut self) -> &mut Self {
        self.numbered = true;
        self
    }

    /// Show author emails alongside names.
    pub fn email(&mut self) -> &mut Self {
        self.email = true;
        self
    }

    /// Group commits by committer instead of author.
    pub fn committer(&mut self) -> &mut Self {
        self.committer = true;
        self
    }

    /// Wrap subject lines at `width` columns, `0` meaning no wrapping.
    ///
    /// git's indent defaults are kept, so wrapped output stays readable by
    /// [`parse_shortlog`](crate::parse::parse_shortlog), which rejoins the
    /// continuation lines.
    pub fn wrap(&mut self, width: usize) -> &mut Self {
        self.wrap = Some(width);
        self
    }

    /// Parse a completed run's [`CommandOutput`] into typed entries.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_entries(&self, output: &CommandOutput) -> Vec<crate::parse::ShortlogEntry> {
        crate::parse::parse_shortlog(&output.stdout_str())
    }
}

#[async_trait]
impl GitCommand for ShortlogCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["shortlog".to_string()];
        if self.summary {
            args.push("-s".into());
        }
        if self.numbered {
            args.push("-n".into());
        }
        if self.email {
            args.push("-e".into());
        }
        if self.committer {
            args.push("-c".into());
        }
        if let Some(width) = self.wrap {
            args.push(format!("-w{width}"));
        }
        args.extend(self.revs.iter().cloned());
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.revs.is_empty() {
            return Err(Error::invalid_config(
                "shortlog: at least one revision is required",
            ));
        }
        self.execute_raw().await
    }
}
