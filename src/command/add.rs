//! `git add` — add file contents to the index.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git add`.
#[derive(Debug, Clone, Default)]
pub struct AddCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Pathspecs to add. `.` or `-A` is common.
    pub paths: Vec<String>,
    /// `--all` / `-A`.
    pub all: bool,
    /// `--update` / `-u`.
    pub update: bool,
    /// `--force` / `-f`.
    pub force: bool,
    /// `--dry-run` / `-n`.
    pub dry_run: bool,
    /// `--verbose` / `-v`.
    pub verbose: bool,
    /// `--intent-to-add` / `-N`.
    pub intent_to_add: bool,
    /// `--patch` / `-p`.
    pub patch: bool,
    /// `--ignore-errors`.
    pub ignore_errors: bool,
}

impl AddCommand {
    /// New empty `add` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a pathspec.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }

    /// Add many pathspecs.
    pub fn paths<I, S>(&mut self, ps: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.paths.extend(ps.into_iter().map(Into::into));
        self
    }

    /// Stage every change, including deletions and new files (`-A`).
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }

    /// Stage only modifications and deletions (`-u`).
    pub fn update(&mut self) -> &mut Self {
        self.update = true;
        self
    }

    /// Override ignore rules.
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }

    /// Show what would happen without changing anything.
    pub fn dry_run(&mut self) -> &mut Self {
        self.dry_run = true;
        self
    }

    /// Be verbose.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }

    /// Record only that a path will be added later.
    pub fn intent_to_add(&mut self) -> &mut Self {
        self.intent_to_add = true;
        self
    }

    /// Interactively stage hunks.
    pub fn patch(&mut self) -> &mut Self {
        self.patch = true;
        self
    }

    /// Continue past files that cannot be added.
    pub fn ignore_errors(&mut self) -> &mut Self {
        self.ignore_errors = true;
        self
    }
}

#[async_trait]
impl GitCommand for AddCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["add".to_string()];
        if self.all {
            args.push("--all".into());
        }
        if self.update {
            args.push("--update".into());
        }
        if self.force {
            args.push("--force".into());
        }
        if self.dry_run {
            args.push("--dry-run".into());
        }
        if self.verbose {
            args.push("--verbose".into());
        }
        if self.intent_to_add {
            args.push("--intent-to-add".into());
        }
        if self.patch {
            args.push("--patch".into());
        }
        if self.ignore_errors {
            args.push("--ignore-errors".into());
        }
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
