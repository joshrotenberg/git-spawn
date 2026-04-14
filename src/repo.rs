//! High-level handle for operating on a git repository.
//!
//! A [`Repository`] is a cheap, cloneable reference to a working tree path.
//! It is the entry point for most users: construct one via [`Repository::open`],
//! [`Repository::init`], or [`Repository::clone`], then call the porcelain
//! methods defined in submodules of this crate to build commands pre-scoped
//! to this repo.

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
