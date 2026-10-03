//! `git status` — show the working tree status.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Porcelain v2 formats and friends.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum StatusFormat {
    /// Short `-s` format.
    Short,
    /// Long (default) format.
    Long,
    /// `--porcelain=v1`.
    PorcelainV1,
    /// `--porcelain=v2`.
    PorcelainV2,
}

/// Submodule handling for `--ignore-submodules=<when>`.
///
/// Passing a mode on the command line overrides the repository's
/// `diff.ignoreSubmodules` and `submodule.<name>.ignore` settings, so
/// [`IgnoreSubmodules::None`] is the way to make a cleanliness check count
/// submodule changes regardless of configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IgnoreSubmodules {
    /// `none`: report a submodule as modified when it has untracked files,
    /// modified files, or a checked-out commit that differs from the one
    /// recorded in the superproject.
    None,
    /// `untracked`: do not count untracked files inside submodules.
    Untracked,
    /// `dirty`: ignore changes to submodule work trees; only a moved
    /// submodule pointer is reported.
    Dirty,
    /// `all`: hide all changes to submodules.
    All,
}

impl IgnoreSubmodules {
    /// The value git expects after `--ignore-submodules=`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Untracked => "untracked",
            Self::Dirty => "dirty",
            Self::All => "all",
        }
    }
}

/// Builder for `git status`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct StatusCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Output format override.
    pub format: Option<StatusFormat>,
    /// `--branch` / `-b`.
    pub branch: bool,
    /// `--show-stash`.
    pub show_stash: bool,
    /// NUL-terminate entries (`-z`).
    pub null_terminate: bool,
    /// `--untracked-files=<mode>`.
    pub untracked_files: Option<String>,
    /// `--ignored`.
    pub ignored: bool,
    /// `--ignore-submodules=<when>`.
    pub ignore_submodules: Option<IgnoreSubmodules>,
    /// Pathspec filters.
    pub paths: Vec<String>,
}

impl StatusCommand {
    /// New status command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set output format.
    pub fn format(&mut self, f: StatusFormat) -> &mut Self {
        self.format = Some(f);
        self
    }

    /// Include branch info.
    pub fn branch(&mut self) -> &mut Self {
        self.branch = true;
        self
    }

    /// Show stash count.
    pub fn show_stash(&mut self) -> &mut Self {
        self.show_stash = true;
        self
    }

    /// NUL-separate entries.
    pub fn null_terminate(&mut self) -> &mut Self {
        self.null_terminate = true;
        self
    }

    /// Set untracked-file mode (`no`, `normal`, `all`).
    pub fn untracked_files(&mut self, mode: impl Into<String>) -> &mut Self {
        self.untracked_files = Some(mode.into());
        self
    }

    /// Also show ignored files.
    pub fn ignored(&mut self) -> &mut Self {
        self.ignored = true;
        self
    }

    /// Set submodule handling (`--ignore-submodules=<when>`), overriding
    /// the repository configuration. Last call wins.
    pub fn ignore_submodules(&mut self, mode: IgnoreSubmodules) -> &mut Self {
        self.ignore_submodules = Some(mode);
        self
    }

    /// Filter by path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }
}

#[async_trait]
impl GitCommand for StatusCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["status".to_string()];
        match self.format {
            Some(StatusFormat::Short) => args.push("--short".into()),
            Some(StatusFormat::Long) => args.push("--long".into()),
            Some(StatusFormat::PorcelainV1) => args.push("--porcelain=v1".into()),
            Some(StatusFormat::PorcelainV2) => args.push("--porcelain=v2".into()),
            None => {}
        }
        if self.branch {
            args.push("--branch".into());
        }
        if self.show_stash {
            args.push("--show-stash".into());
        }
        if self.null_terminate {
            args.push("-z".into());
        }
        if let Some(m) = &self.untracked_files {
            args.push(format!("--untracked-files={m}"));
        }
        if self.ignored {
            args.push("--ignored".into());
        }
        if let Some(m) = self.ignore_submodules {
            args.push(format!("--ignore-submodules={}", m.as_str()));
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
