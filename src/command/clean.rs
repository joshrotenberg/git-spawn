//! `git clean` — remove untracked files from the working tree.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git clean`.
///
/// Removes files that are not tracked by git. By default only untracked
/// files in the current directory are considered: [`directories`](Self::directories)
/// (`-d`) extends that to untracked directories, and [`ignored`](Self::ignored)
/// (`-x`) stops honouring the ignore rules so ignored files are removed too.
///
/// git refuses to delete anything unless [`force`](Self::force) or
/// [`dry_run`](Self::dry_run) is set, and fails with a message naming
/// `clean.requireForce`. That config key is the only way to opt out, so this
/// builder passes the flags through rather than rejecting the combination
/// itself.
///
/// Pathspecs restrict the operation to matching paths and are passed after a
/// `--` separator so a leading dash cannot be read as an option.
///
/// Output is left as a [`CommandOutput`]; each affected path is reported on
/// stdout as `Removing <path>`, or `Would remove <path>` under `-n`.
#[derive(Debug, Clone, Default)]
pub struct CleanCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Pathspecs limiting which untracked files are considered.
    pub paths: Vec<String>,
    /// `-f`: actually remove the files.
    pub force: bool,
    /// `-n`: report what would be removed without removing it.
    pub dry_run: bool,
    /// `-d`: recurse into untracked directories.
    pub directories: bool,
    /// `-x`: also remove ignored files.
    pub ignored: bool,
}

impl CleanCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Limit the operation to a pathspec.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }

    /// Limit the operation to many pathspecs.
    pub fn paths<I, S>(&mut self, ps: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.paths.extend(ps.into_iter().map(Into::into));
        self
    }

    /// Remove the files (`-f`).
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }

    /// Report what would be removed, removing nothing (`-n`).
    pub fn dry_run(&mut self) -> &mut Self {
        self.dry_run = true;
        self
    }

    /// Recurse into untracked directories (`-d`).
    pub fn directories(&mut self) -> &mut Self {
        self.directories = true;
        self
    }

    /// Also remove ignored files (`-x`).
    pub fn ignored(&mut self) -> &mut Self {
        self.ignored = true;
        self
    }
}

#[async_trait]
impl GitCommand for CleanCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["clean".to_string()];
        if self.force {
            args.push("--force".into());
        }
        if self.dry_run {
            args.push("--dry-run".into());
        }
        if self.directories {
            args.push("-d".into());
        }
        if self.ignored {
            args.push("-x".into());
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
