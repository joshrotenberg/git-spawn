//! `git ls-files` — show information about files in the index and working tree.
//!
//! ```no_run
//! use git_spawn::{GitCommand, LsFilesCommand};
//!
//! # async fn example() -> git_spawn::Result<()> {
//! let mut cmd = LsFilesCommand::new();
//! cmd.current_dir("/repo").cached();
//! let out = cmd.execute().await?;
//! for path in out.stdout_str().lines() {
//!     println!("{path}");
//! }
//! # Ok(())
//! # }
//! ```

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git ls-files`.
#[derive(Debug, Clone, Default)]
pub struct LsFilesCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `--cached` / `-c`.
    pub cached: bool,
    /// `--deleted` / `-d`.
    pub deleted: bool,
    /// `--modified` / `-m`.
    pub modified: bool,
    /// `--others` / `-o`.
    pub others: bool,
    /// `--ignored` / `-i`.
    pub ignored: bool,
    /// `--stage` / `-s`.
    pub stage: bool,
    /// `--unmerged` / `-u`.
    pub unmerged: bool,
    /// `--exclude-standard`.
    pub exclude_standard: bool,
    /// `-z` NUL-terminate.
    pub null_terminate: bool,
    /// Pathspec filters.
    pub paths: Vec<String>,
}

impl LsFilesCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Show cached files (default behavior).
    pub fn cached(&mut self) -> &mut Self {
        self.cached = true;
        self
    }

    /// Show deleted files.
    pub fn deleted(&mut self) -> &mut Self {
        self.deleted = true;
        self
    }

    /// Show modified files.
    pub fn modified(&mut self) -> &mut Self {
        self.modified = true;
        self
    }

    /// Show untracked (other) files.
    pub fn others(&mut self) -> &mut Self {
        self.others = true;
        self
    }

    /// Show ignored files (combine with `--others`).
    pub fn ignored(&mut self) -> &mut Self {
        self.ignored = true;
        self
    }

    /// Include stage numbers and SHAs.
    pub fn stage(&mut self) -> &mut Self {
        self.stage = true;
        self
    }

    /// Show unmerged files only.
    pub fn unmerged(&mut self) -> &mut Self {
        self.unmerged = true;
        self
    }

    /// Honor `.gitignore` when listing others.
    pub fn exclude_standard(&mut self) -> &mut Self {
        self.exclude_standard = true;
        self
    }

    /// NUL-terminate output (useful for paths with newlines).
    pub fn null_terminate(&mut self) -> &mut Self {
        self.null_terminate = true;
        self
    }

    /// Filter by path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }
}

#[async_trait]
impl GitCommand for LsFilesCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["ls-files".to_string()];
        if self.cached {
            args.push("--cached".into());
        }
        if self.deleted {
            args.push("--deleted".into());
        }
        if self.modified {
            args.push("--modified".into());
        }
        if self.others {
            args.push("--others".into());
        }
        if self.ignored {
            args.push("--ignored".into());
        }
        if self.stage {
            args.push("--stage".into());
        }
        if self.unmerged {
            args.push("--unmerged".into());
        }
        if self.exclude_standard {
            args.push("--exclude-standard".into());
        }
        if self.null_terminate {
            args.push("-z".into());
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
