//! `git blame` — show what revision and author last touched each line.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Machine-readable output formats for `git blame`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlameFormat {
    /// `--porcelain`: commit metadata written once per commit.
    Porcelain,
    /// `--line-porcelain`: commit metadata repeated on every line.
    LinePorcelain,
}

/// Builder for `git blame`.
///
/// The path is required: `git blame` with no file annotates nothing, so
/// [`execute`](GitCommand::execute) rejects a missing one as
/// [`Error::InvalidConfig`] rather than letting git print its usage.
///
/// The default output is the human-readable format, whose columns shift with
/// terminal width, commit abbreviation length, and date settings. Ask for
/// [`porcelain`](Self::porcelain) or [`line_porcelain`](Self::line_porcelain)
/// before calling [`parse_entries`](Self::parse_entries); those are the two
/// formats [`parse_blame`](crate::parse::parse_blame) understands.
///
/// Output is left as a [`CommandOutput`].
#[derive(Debug, Clone, Default)]
pub struct BlameCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The file to annotate.
    pub file: Option<String>,
    /// Revision to blame, defaulting to the working tree inside git.
    pub rev: Option<String>,
    /// `-L <start>,<end>`: restrict output to a line range.
    pub line_range: Option<(u32, u32)>,
    /// Machine-readable output format.
    pub format: Option<BlameFormat>,
    /// `-e`: show author emails instead of names.
    pub show_email: bool,
    /// `-w`: ignore whitespace-only changes when assigning blame.
    pub ignore_whitespace: bool,
    /// `-M`: detect lines moved within the file.
    pub detect_moved: bool,
    /// `-C`: detect lines copied from other files in the same commit.
    pub detect_copied: bool,
}

impl BlameCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Annotate `file`.
    pub fn file(&mut self, file: impl Into<String>) -> &mut Self {
        self.file = Some(file.into());
        self
    }

    /// Blame `rev` instead of the working tree.
    pub fn rev(&mut self, rev: impl Into<String>) -> &mut Self {
        self.rev = Some(rev.into());
        self
    }

    /// Restrict output to lines `start` through `end`, both inclusive and
    /// 1-based.
    pub fn lines(&mut self, start: u32, end: u32) -> &mut Self {
        self.line_range = Some((start, end));
        self
    }

    /// Emit `--porcelain` output.
    pub fn porcelain(&mut self) -> &mut Self {
        self.format = Some(BlameFormat::Porcelain);
        self
    }

    /// Emit `--line-porcelain` output.
    pub fn line_porcelain(&mut self) -> &mut Self {
        self.format = Some(BlameFormat::LinePorcelain);
        self
    }

    /// Show author emails instead of names.
    pub fn show_email(&mut self) -> &mut Self {
        self.show_email = true;
        self
    }

    /// Ignore whitespace-only changes.
    pub fn ignore_whitespace(&mut self) -> &mut Self {
        self.ignore_whitespace = true;
        self
    }

    /// Detect lines moved within the file.
    pub fn detect_moved(&mut self) -> &mut Self {
        self.detect_moved = true;
        self
    }

    /// Detect lines copied from other files modified in the same commit.
    pub fn detect_copied(&mut self) -> &mut Self {
        self.detect_copied = true;
        self
    }

    /// Parse a completed run's [`CommandOutput`] into typed entries.
    ///
    /// The run must have used [`porcelain`](Self::porcelain) or
    /// [`line_porcelain`](Self::line_porcelain); any other format parses to an
    /// empty list.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_entries(&self, output: &CommandOutput) -> Vec<crate::parse::BlameEntry> {
        crate::parse::parse_blame(&output.stdout_str())
    }
}

#[async_trait]
impl GitCommand for BlameCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["blame".to_string()];
        match self.format {
            Some(BlameFormat::Porcelain) => args.push("--porcelain".into()),
            Some(BlameFormat::LinePorcelain) => args.push("--line-porcelain".into()),
            None => {}
        }
        if self.show_email {
            args.push("-e".into());
        }
        if self.ignore_whitespace {
            args.push("-w".into());
        }
        if self.detect_moved {
            args.push("-M".into());
        }
        if self.detect_copied {
            args.push("-C".into());
        }
        if let Some((start, end)) = self.line_range {
            args.push("-L".into());
            args.push(format!("{start},{end}"));
        }
        if let Some(rev) = &self.rev {
            args.push(rev.clone());
        }
        if let Some(file) = &self.file {
            args.push("--".into());
            args.push(file.clone());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.file.is_none() {
            return Err(Error::invalid_config("blame: a file is required"));
        }
        if let Some((start, end)) = self.line_range {
            if start == 0 {
                return Err(Error::invalid_config("blame: line numbers are 1-based"));
            }
            if end < start {
                return Err(Error::invalid_config(
                    "blame: the line range ends before it starts",
                ));
            }
        }
        self.execute_raw().await
    }
}
