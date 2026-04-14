//! `git commit` — record changes to the repository.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Builder for `git commit`.
#[derive(Debug, Clone, Default)]
pub struct CommitCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `-m`.
    pub message: Option<String>,
    /// `-F` file.
    pub message_file: Option<PathBuf>,
    /// `--all` / `-a`.
    pub all: bool,
    /// `--amend`.
    pub amend: bool,
    /// `--no-edit`.
    pub no_edit: bool,
    /// `--allow-empty`.
    pub allow_empty: bool,
    /// `--allow-empty-message`.
    pub allow_empty_message: bool,
    /// `--signoff` / `-s`.
    pub signoff: bool,
    /// `--no-verify`.
    pub no_verify: bool,
    /// `--author`.
    pub author: Option<String>,
    /// `--date`.
    pub date: Option<String>,
    /// `--only` specific paths.
    pub only_paths: Vec<String>,
    /// `--quiet`.
    pub quiet: bool,
    /// `--verbose`.
    pub verbose: bool,
}

impl CommitCommand {
    /// Build a bare commit command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience: commit with a message.
    pub fn with_message(msg: impl Into<String>) -> Self {
        let mut c = Self::new();
        c.message = Some(msg.into());
        c
    }

    /// Set the commit message (`-m`).
    pub fn message(&mut self, msg: impl Into<String>) -> &mut Self {
        self.message = Some(msg.into());
        self
    }

    /// Read the commit message from a file (`-F`).
    pub fn message_file(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.message_file = Some(path.into());
        self
    }

    /// Stage all tracked, modified files before committing (`-a`).
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }

    /// Amend the previous commit.
    pub fn amend(&mut self) -> &mut Self {
        self.amend = true;
        self
    }

    /// Reuse the last commit message without opening the editor.
    pub fn no_edit(&mut self) -> &mut Self {
        self.no_edit = true;
        self
    }

    /// Allow an empty commit.
    pub fn allow_empty(&mut self) -> &mut Self {
        self.allow_empty = true;
        self
    }

    /// Allow an empty commit message.
    pub fn allow_empty_message(&mut self) -> &mut Self {
        self.allow_empty_message = true;
        self
    }

    /// Add `Signed-off-by:` line.
    pub fn signoff(&mut self) -> &mut Self {
        self.signoff = true;
        self
    }

    /// Skip pre-commit and commit-msg hooks.
    pub fn no_verify(&mut self) -> &mut Self {
        self.no_verify = true;
        self
    }

    /// Override the author (`--author="Name <email>"`).
    pub fn author(&mut self, a: impl Into<String>) -> &mut Self {
        self.author = Some(a.into());
        self
    }

    /// Override the author date.
    pub fn date(&mut self, d: impl Into<String>) -> &mut Self {
        self.date = Some(d.into());
        self
    }

    /// Commit only the given paths.
    pub fn only(&mut self, path: impl Into<String>) -> &mut Self {
        self.only_paths.push(path.into());
        self
    }

    /// Suppress output.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }

    /// Verbose diff output in the commit message editor.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }
}

#[async_trait]
impl GitCommand for CommitCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["commit".to_string()];
        if self.all {
            args.push("--all".into());
        }
        if self.amend {
            args.push("--amend".into());
        }
        if self.no_edit {
            args.push("--no-edit".into());
        }
        if self.allow_empty {
            args.push("--allow-empty".into());
        }
        if self.allow_empty_message {
            args.push("--allow-empty-message".into());
        }
        if self.signoff {
            args.push("--signoff".into());
        }
        if self.no_verify {
            args.push("--no-verify".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if self.verbose {
            args.push("--verbose".into());
        }
        if let Some(a) = &self.author {
            args.push(format!("--author={a}"));
        }
        if let Some(d) = &self.date {
            args.push(format!("--date={d}"));
        }
        if let Some(m) = &self.message {
            args.push("-m".into());
            args.push(m.clone());
        }
        if let Some(f) = &self.message_file {
            args.push("-F".into());
            args.push(f.display().to_string());
        }
        if !self.only_paths.is_empty() {
            args.push("--only".into());
            args.push("--".into());
            args.extend(self.only_paths.iter().cloned());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
