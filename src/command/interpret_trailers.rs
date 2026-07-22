//! `git interpret-trailers` — add or inspect trailers in a commit message.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// Where a new trailer is placed relative to the existing ones (`--where`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailerWhere {
    /// `after`: directly after the last matching trailer.
    After,
    /// `before`: directly before the first matching trailer.
    Before,
    /// `end`: at the end of the trailer block.
    End,
    /// `start`: at the start of the trailer block.
    Start,
}

impl TrailerWhere {
    /// The value git expects for `--where`.
    fn as_str(self) -> &'static str {
        match self {
            Self::After => "after",
            Self::Before => "before",
            Self::End => "end",
            Self::Start => "start",
        }
    }
}

/// What to do when a trailer with the same token is already present
/// (`--if-exists`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailerIfExists {
    /// `addIfDifferent`: add unless an identical trailer is already there.
    AddIfDifferent,
    /// `addIfDifferentNeighbor`: add unless an identical trailer is adjacent.
    AddIfDifferentNeighbor,
    /// `add`: add regardless of what is already there.
    Add,
    /// `replace`: drop the existing trailer and add the new one.
    Replace,
    /// `doNothing`: leave the message unchanged.
    DoNothing,
}

impl TrailerIfExists {
    /// The value git expects for `--if-exists`.
    fn as_str(self) -> &'static str {
        match self {
            Self::AddIfDifferent => "addIfDifferent",
            Self::AddIfDifferentNeighbor => "addIfDifferentNeighbor",
            Self::Add => "add",
            Self::Replace => "replace",
            Self::DoNothing => "doNothing",
        }
    }
}

/// What to do when no trailer with the same token is present (`--if-missing`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrailerIfMissing {
    /// `doNothing`: leave the message unchanged.
    DoNothing,
    /// `add`: add the trailer.
    Add,
}

impl TrailerIfMissing {
    /// The value git expects for `--if-missing`.
    fn as_str(self) -> &'static str {
        match self {
            Self::DoNothing => "doNothing",
            Self::Add => "add",
        }
    }
}

/// Builder for `git interpret-trailers`.
///
/// Adds trailers such as `Signed-off-by` to a commit message, or reads the
/// trailers already in one. Messages are passed as file paths: reading a
/// message from stdin is not modelled, so [`execute`](GitCommand::execute)
/// rejects an empty file list rather than handing git a command that would
/// block on the parent's stdin.
///
/// By default git writes the resulting message to stdout; [`in_place`](Self::in_place)
/// rewrites the input file instead.
#[derive(Debug, Clone, Default)]
pub struct InterpretTrailersCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The message files to process, in the order they were added.
    pub files: Vec<PathBuf>,
    /// `--trailer`: the trailers to add, in the order they were added.
    pub trailers: Vec<String>,
    /// `--in-place`: edit the input files rather than writing to stdout.
    pub in_place: bool,
    /// `--trim-empty`: drop trailers whose value is empty.
    pub trim_empty: bool,
    /// `--only-trailers`: output the trailer block only.
    pub only_trailers: bool,
    /// `--only-input`: output only the trailers found in the input.
    pub only_input: bool,
    /// `--unfold`: join each multi-line trailer onto one line.
    pub unfold: bool,
    /// `--parse`: shorthand for `--only-trailers --only-input --unfold`.
    pub parse: bool,
    /// `--no-divider`: do not treat `---` as terminating the message.
    pub no_divider: bool,
    /// `--where`: placement of a new trailer.
    pub placement: Option<TrailerWhere>,
    /// `--if-exists`: behaviour when the token is already present.
    pub if_exists: Option<TrailerIfExists>,
    /// `--if-missing`: behaviour when the token is absent.
    pub if_missing: Option<TrailerIfMissing>,
}

impl InterpretTrailersCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Process the message at `path`. Call repeatedly to process several files.
    pub fn file(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.files.push(path.into());
        self
    }

    /// Add the trailer `token: value`.
    pub fn trailer(&mut self, token: impl AsRef<str>, value: impl AsRef<str>) -> &mut Self {
        self.trailers
            .push(format!("{}: {}", token.as_ref(), value.as_ref()));
        self
    }

    /// Add a trailer already spelled the way git accepts it, for example
    /// `Signed-off-by: A U Thor <author@example.com>` or a configured shorthand
    /// such as `sign=A U Thor <author@example.com>`.
    pub fn trailer_raw(&mut self, trailer: impl Into<String>) -> &mut Self {
        self.trailers.push(trailer.into());
        self
    }

    /// Edit the input files in place instead of writing to stdout.
    pub fn in_place(&mut self) -> &mut Self {
        self.in_place = true;
        self
    }

    /// Drop trailers whose value is empty.
    pub fn trim_empty(&mut self) -> &mut Self {
        self.trim_empty = true;
        self
    }

    /// Output the trailer block only.
    pub fn only_trailers(&mut self) -> &mut Self {
        self.only_trailers = true;
        self
    }

    /// Output only the trailers found in the input, ignoring configured ones.
    pub fn only_input(&mut self) -> &mut Self {
        self.only_input = true;
        self
    }

    /// Join each multi-line trailer onto a single line.
    pub fn unfold(&mut self) -> &mut Self {
        self.unfold = true;
        self
    }

    /// Shorthand for `--only-trailers --only-input --unfold`.
    pub fn parse(&mut self) -> &mut Self {
        self.parse = true;
        self
    }

    /// Do not treat a `---` line as the end of the message.
    pub fn no_divider(&mut self) -> &mut Self {
        self.no_divider = true;
        self
    }

    /// Where to place a new trailer relative to the existing ones.
    pub fn placement(&mut self, placement: TrailerWhere) -> &mut Self {
        self.placement = Some(placement);
        self
    }

    /// What to do when a trailer with the same token is already present.
    pub fn if_exists(&mut self, action: TrailerIfExists) -> &mut Self {
        self.if_exists = Some(action);
        self
    }

    /// What to do when no trailer with the same token is present.
    pub fn if_missing(&mut self, action: TrailerIfMissing) -> &mut Self {
        self.if_missing = Some(action);
        self
    }
}

#[async_trait]
impl GitCommand for InterpretTrailersCommand {
    /// Raw output. The rewritten message (or, with `--in-place`, nothing)
    /// arrives on stdout.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["interpret-trailers".to_string()];
        if self.in_place {
            args.push("--in-place".into());
        }
        if self.trim_empty {
            args.push("--trim-empty".into());
        }
        if self.parse {
            args.push("--parse".into());
        }
        if self.only_trailers {
            args.push("--only-trailers".into());
        }
        if self.only_input {
            args.push("--only-input".into());
        }
        if self.unfold {
            args.push("--unfold".into());
        }
        if self.no_divider {
            args.push("--no-divider".into());
        }
        if let Some(placement) = self.placement {
            args.push(format!("--where={}", placement.as_str()));
        }
        if let Some(action) = self.if_exists {
            args.push(format!("--if-exists={}", action.as_str()));
        }
        if let Some(action) = self.if_missing {
            args.push(format!("--if-missing={}", action.as_str()));
        }
        for trailer in &self.trailers {
            args.push("--trailer".into());
            args.push(trailer.clone());
        }
        for file in &self.files {
            args.push(file.display().to_string());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.files.is_empty() {
            return Err(Error::invalid_config(
                "interpret-trailers requires at least one message file; reading from stdin is not supported",
            ));
        }
        self.execute_raw().await
    }
}
