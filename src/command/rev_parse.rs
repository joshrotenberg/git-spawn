//! `git rev-parse` — pick out and massage parameters.
//!
//! `rev-parse` is the swiss army knife of git plumbing: resolve refs to SHAs,
//! query the `.git` directory, show the top-level, check whether the cwd is
//! inside a working tree, etc. This wrapper exposes the common modes and
//! returns stdout trimmed as [`String`] so callers can parse as needed.
//!
//! ```no_run
//! use git_spawn::{GitCommand, RevParseCommand};
//!
//! # async fn example() -> git_spawn::Result<()> {
//! let mut cmd = RevParseCommand::new();
//! cmd.arg_str("HEAD").current_dir("/some/repo");
//! let sha = cmd.execute().await?;
//! println!("HEAD -> {sha}");
//! # Ok(())
//! # }
//! ```

use crate::command::{CommandExecutor, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// How [`RevParseCommand::git_common_dir`] and
/// [`absolute_git_common_dir`] spell the path they print.
///
/// [`absolute_git_common_dir`]: RevParseCommand::absolute_git_common_dir
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GitCommonDirFormat {
    /// `--git-common-dir` on its own: git prints whatever spelling it has,
    /// which is relative (`.git`) from the main working tree but absolute from
    /// a linked one.
    AsIs,
    /// `--path-format=absolute --git-common-dir`: always an absolute,
    /// canonical path.
    Absolute,
}

/// Builder for `git rev-parse`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RevParseCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Arguments / refs / flags to pass to `rev-parse`.
    pub rev_args: Vec<String>,
    /// `--verify`.
    pub verify: bool,
    /// `--abbrev-ref`.
    pub abbrev_ref: bool,
    /// `--short[=N]`.
    pub short: Option<Option<u32>>,
    /// `--show-toplevel`.
    pub show_toplevel: bool,
    /// `--git-dir`.
    pub git_dir: bool,
    /// `--is-inside-work-tree`.
    pub is_inside_work_tree: bool,
    /// `--is-bare-repository`.
    pub is_bare_repository: bool,
    /// `--absolute-git-dir`.
    pub absolute_git_dir: bool,
    /// `--git-common-dir`, and the path spelling to ask for.
    pub git_common_dir: Option<GitCommonDirFormat>,
}

impl RevParseCommand {
    /// New empty `rev-parse` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a ref/rev string (e.g. `"HEAD"`, `"main"`, `"HEAD~3"`).
    pub fn arg_str(&mut self, s: impl Into<String>) -> &mut Self {
        self.rev_args.push(s.into());
        self
    }

    /// `--verify`: error if the argument is not a valid object.
    pub fn verify(&mut self) -> &mut Self {
        self.verify = true;
        self
    }

    /// `--abbrev-ref`: print the short ref name.
    pub fn abbrev_ref(&mut self) -> &mut Self {
        self.abbrev_ref = true;
        self
    }

    /// `--short` with default length.
    pub fn short(&mut self) -> &mut Self {
        self.short = Some(None);
        self
    }

    /// `--short=N`.
    pub fn short_len(&mut self, n: u32) -> &mut Self {
        self.short = Some(Some(n));
        self
    }

    /// `--show-toplevel`.
    pub fn show_toplevel(&mut self) -> &mut Self {
        self.show_toplevel = true;
        self
    }

    /// `--git-dir`.
    pub fn git_dir(&mut self) -> &mut Self {
        self.git_dir = true;
        self
    }

    /// `--absolute-git-dir`.
    pub fn absolute_git_dir(&mut self) -> &mut Self {
        self.absolute_git_dir = true;
        self
    }

    /// `--git-common-dir`: the `.git` directory shared by every working tree.
    ///
    /// This is the query to use when the caller may be holding a *linked*
    /// worktree: [`git_dir`] there names the worktree's own private
    /// `.git/worktrees/<name>` directory, while this names the shared
    /// repository directory. From the main working tree the two agree.
    ///
    /// Git prints whatever spelling it has, so the result is relative from the
    /// main working tree and absolute from a linked one. Use
    /// [`absolute_git_common_dir`] when you need an absolute path either way.
    /// Replaces any earlier [`absolute_git_common_dir`] call.
    ///
    /// [`git_dir`]: RevParseCommand::git_dir
    /// [`absolute_git_common_dir`]: RevParseCommand::absolute_git_common_dir
    pub fn git_common_dir(&mut self) -> &mut Self {
        self.git_common_dir = Some(GitCommonDirFormat::AsIs);
        self
    }

    /// `--path-format=absolute --git-common-dir`: the shared `.git` directory
    /// as an absolute, canonical path.
    ///
    /// Replaces any earlier [`git_common_dir`] call.
    ///
    /// [`git_common_dir`]: RevParseCommand::git_common_dir
    pub fn absolute_git_common_dir(&mut self) -> &mut Self {
        self.git_common_dir = Some(GitCommonDirFormat::Absolute);
        self
    }

    /// `--is-inside-work-tree`.
    pub fn is_inside_work_tree(&mut self) -> &mut Self {
        self.is_inside_work_tree = true;
        self
    }

    /// `--is-bare-repository`.
    pub fn is_bare_repository(&mut self) -> &mut Self {
        self.is_bare_repository = true;
        self
    }
}

#[async_trait]
impl GitCommand for RevParseCommand {
    /// Trimmed stdout — typically a SHA, path, or `true`/`false` string.
    type Output = String;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["rev-parse".to_string()];
        if self.verify {
            args.push("--verify".into());
        }
        if self.abbrev_ref {
            args.push("--abbrev-ref".into());
        }
        match self.short {
            Some(None) => args.push("--short".into()),
            Some(Some(n)) => args.push(format!("--short={n}")),
            None => {}
        }
        if self.show_toplevel {
            args.push("--show-toplevel".into());
        }
        if self.git_dir {
            args.push("--git-dir".into());
        }
        if self.absolute_git_dir {
            args.push("--absolute-git-dir".into());
        }
        // `--path-format` is positional: it rewrites only the options that
        // follow it. Emitting the pair after the other path queries keeps it
        // from silently making `--git-dir` and `--show-toplevel` absolute too.
        match self.git_common_dir {
            Some(GitCommonDirFormat::AsIs) => args.push("--git-common-dir".into()),
            Some(GitCommonDirFormat::Absolute) => {
                args.push("--path-format=absolute".into());
                args.push("--git-common-dir".into());
            }
            None => {}
        }
        if self.is_inside_work_tree {
            args.push("--is-inside-work-tree".into());
        }
        if self.is_bare_repository {
            args.push("--is-bare-repository".into());
        }
        args.extend(self.rev_args.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<String> {
        let out = self.execute_raw().await?;
        Ok(out.stdout_trimmed())
    }
}
