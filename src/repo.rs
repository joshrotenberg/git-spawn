//! High-level handle for operating on a git repository.
//!
//! A [`Repository`] is a cheap, cloneable reference to a working tree path.
//! It is the entry point for most users: construct one via
//! [`Repository::open`], [`Repository::init`], or [`Repository::clone`], then
//! call the accessor methods ([`Repository::add`], [`Repository::commit`],
//! [`Repository::log`], ...) to build commands pre-scoped to this repo.
//!
//! ```no_run
//! use git_spawn::{GitCommand, Repository};
//!
//! # async fn example() -> git_spawn::Result<()> {
//! // Create a fresh repo and commit a file into it.
//! let repo = Repository::init("/tmp/demo").await?;
//! std::fs::write(repo.path().join("hello.txt"), "hi")?;
//! repo.add().path("hello.txt").execute().await?;
//! repo.commit().message("first").execute().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Cloning an existing repo
//!
//! ```no_run
//! # use git_spawn::Repository;
//! # async fn example() -> git_spawn::Result<()> {
//! let repo = Repository::clone(
//!     "https://github.com/octocat/Hello-World.git",
//!     "/tmp/hello-world",
//! ).await?;
//! assert!(repo.git_dir().exists());
//! # Ok(())
//! # }
//! ```

use crate::command::{
    GitCommand, add::AddCommand, bisect::BisectCommand, branch::BranchCommand,
    checkout::CheckoutCommand, cherry_pick::CherryPickCommand, clone::CloneCommand,
    commit::CommitCommand, config::ConfigCommand, diff::DiffCommand, fetch::FetchCommand,
    grep::GrepCommand, init::InitCommand, log::LogCommand, merge::MergeCommand, mv::MvCommand,
    pull::PullCommand, push::PushCommand, rebase::RebaseCommand, reflog::ReflogCommand,
    remote::RemoteCommand, reset::ResetCommand, restore::RestoreCommand, rm::RmCommand,
    show::ShowCommand, stash::StashCommand, status::StatusCommand, submodule::SubmoduleCommand,
    switch::SwitchCommand, tag::TagCommand, worktree::WorktreeCommand,
};
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// A handle to a git working tree.
///
/// Construction does not spawn `git`. [`Repository::open`] only verifies that
/// a `.git` directory (or file, for worktrees/submodules) exists at the path.
#[derive(Debug, Clone)]
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    /// Open an existing repository at `path` without running `git`.
    ///
    /// Returns [`Error::NotARepository`] if `path/.git` does not exist.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let dotgit = path.join(".git");
        if !dotgit.exists() {
            return Err(Error::not_a_repository(path.display().to_string()));
        }
        Ok(Self { path })
    }

    /// Construct a [`Repository`] for `path` without checking that it exists.
    ///
    /// Use this when you are about to run `init` or `clone` into the path.
    #[must_use]
    pub fn new_unchecked(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Working-tree path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path to the `.git` directory (or file) inside the working tree.
    #[must_use]
    pub fn git_dir(&self) -> PathBuf {
        self.path.join(".git")
    }

    /// Initialize a new repository at `path`.
    ///
    /// Equivalent to `git init <path>`. Returns the created [`Repository`].
    pub async fn init(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).map_err(Error::from)?;
            }
        }
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(Error::from)?;
        }
        InitCommand::in_directory(path).execute().await
    }

    /// Clone `url` into `path`.
    pub async fn clone(url: impl Into<String>, path: impl Into<PathBuf>) -> Result<Self> {
        let mut cmd = CloneCommand::new(url);
        cmd.directory(path);
        cmd.execute().await
    }

    /// Build an [`AddCommand`] scoped to this repository.
    #[must_use]
    pub fn add(&self) -> AddCommand {
        let mut c = AddCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`CommitCommand`] scoped to this repository.
    #[must_use]
    pub fn commit(&self) -> CommitCommand {
        let mut c = CommitCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`StatusCommand`] scoped to this repository.
    #[must_use]
    pub fn status(&self) -> StatusCommand {
        let mut c = StatusCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`LogCommand`] scoped to this repository.
    #[must_use]
    pub fn log(&self) -> LogCommand {
        let mut c = LogCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`DiffCommand`] scoped to this repository.
    #[must_use]
    pub fn diff(&self) -> DiffCommand {
        let mut c = DiffCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ShowCommand`] scoped to this repository.
    #[must_use]
    pub fn show(&self) -> ShowCommand {
        let mut c = ShowCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`BranchCommand`] scoped to this repository.
    #[must_use]
    pub fn branch(&self) -> BranchCommand {
        let mut c = BranchCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`CheckoutCommand`] scoped to this repository.
    #[must_use]
    pub fn checkout(&self) -> CheckoutCommand {
        let mut c = CheckoutCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`SwitchCommand`] scoped to this repository.
    #[must_use]
    pub fn switch(&self) -> SwitchCommand {
        let mut c = SwitchCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`MergeCommand`] scoped to this repository.
    #[must_use]
    pub fn merge(&self) -> MergeCommand {
        let mut c = MergeCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`RebaseCommand`] scoped to this repository.
    #[must_use]
    pub fn rebase(&self) -> RebaseCommand {
        let mut c = RebaseCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`PullCommand`] scoped to this repository.
    #[must_use]
    pub fn pull(&self) -> PullCommand {
        let mut c = PullCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`PushCommand`] scoped to this repository.
    #[must_use]
    pub fn push(&self) -> PushCommand {
        let mut c = PushCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`FetchCommand`] scoped to this repository.
    #[must_use]
    pub fn fetch(&self) -> FetchCommand {
        let mut c = FetchCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`RemoteCommand`] scoped to this repository.
    #[must_use]
    pub fn remote(&self, action: RemoteCommand) -> RemoteCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`TagCommand`] scoped to this repository.
    #[must_use]
    pub fn tag(&self) -> TagCommand {
        let mut c = TagCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`StashCommand`] scoped to this repository.
    #[must_use]
    pub fn stash(&self, action: StashCommand) -> StashCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ResetCommand`] scoped to this repository.
    #[must_use]
    pub fn reset(&self) -> ResetCommand {
        let mut c = ResetCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`RestoreCommand`] scoped to this repository.
    #[must_use]
    pub fn restore(&self) -> RestoreCommand {
        let mut c = RestoreCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`RmCommand`] scoped to this repository.
    #[must_use]
    pub fn rm(&self) -> RmCommand {
        let mut c = RmCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`MvCommand`] scoped to this repository.
    pub fn mv(&self, src: impl Into<String>, dst: impl Into<String>) -> MvCommand {
        let mut c = MvCommand::new(src, dst);
        c.current_dir(&self.path);
        c
    }

    /// Build a [`CherryPickCommand`] scoped to this repository.
    #[must_use]
    pub fn cherry_pick(&self) -> CherryPickCommand {
        let mut c = CherryPickCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`GrepCommand`] scoped to this repository with the given pattern.
    pub fn grep(&self, pattern: impl Into<String>) -> GrepCommand {
        let mut c = GrepCommand::new(pattern);
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ConfigCommand`] scoped to this repository.
    #[must_use]
    pub fn config(&self, action: ConfigCommand) -> ConfigCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ReflogCommand`] scoped to this repository.
    #[must_use]
    pub fn reflog(&self, action: ReflogCommand) -> ReflogCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`WorktreeCommand`] scoped to this repository.
    #[must_use]
    pub fn worktree(&self, action: WorktreeCommand) -> WorktreeCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`SubmoduleCommand`] scoped to this repository.
    #[must_use]
    pub fn submodule(&self, action: SubmoduleCommand) -> SubmoduleCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`BisectCommand`] scoped to this repository.
    #[must_use]
    pub fn bisect(&self, action: BisectCommand) -> BisectCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_missing_repo_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = Repository::open(tmp.path()).unwrap_err();
        assert!(matches!(err, Error::NotARepository { .. }));
    }

    #[test]
    fn new_unchecked_does_not_check() {
        let repo = Repository::new_unchecked("/definitely/not/here");
        assert_eq!(repo.path(), Path::new("/definitely/not/here"));
    }
}
