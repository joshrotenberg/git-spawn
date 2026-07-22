//! Typed working-tree and index change analysis.
//!
//! Reached through [`Repository::changes`], which returns a [`ChangesOps`]
//! handle. [`summary`](ChangesOps::summary) runs
//! `git status --porcelain=v1 -b -z` once and folds the result into a typed
//! [`Changes`]: the staged, unstaged, and untracked path lists plus the
//! branch/tracking header and its ahead/behind counts.
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//! let changes = repo.changes().summary().await?;
//!
//! if changes.is_dirty() {
//!     println!("{} staged, {} unstaged, {} untracked",
//!         changes.staged.len(), changes.unstaged.len(), changes.untracked.len());
//! }
//! if changes.behind > 0 {
//!     println!("behind {} by {}", changes.tracking.as_deref().unwrap_or("upstream"), changes.behind);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The porcelain status output already separates the index ("X") and
//! working-tree ("Y") columns, so a single `git status` call classifies every
//! path; there is no second `git diff` pass. A path modified in both the index
//! and the working tree (porcelain `MM`) appears in both [`staged`] and
//! [`unstaged`], matching how git itself reports it.
//!
//! [`staged`]: Changes::staged
//! [`unstaged`]: Changes::unstaged

use crate::command::GitCommand;
use crate::command::status::StatusFormat;
use crate::error::Result;
use crate::parse::{Status, StatusKind, parse_full_status};
use crate::repo::Repository;

/// A typed summary of the working tree and index relative to `HEAD` and the
/// upstream branch.
///
/// Produced by [`ChangesOps::summary`]. The three path lists are disjoint by
/// origin, not by file: an index-and-worktree change lands in both [`staged`]
/// and [`unstaged`].
///
/// [`staged`]: Changes::staged
/// [`unstaged`]: Changes::unstaged
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Changes {
    /// Current branch name. `None` for a detached `HEAD`.
    pub branch: Option<String>,
    /// Upstream tracking ref (e.g. `origin/main`), if one is configured.
    pub tracking: Option<String>,
    /// Commits the local branch is ahead of `tracking` by.
    pub ahead: u32,
    /// Commits the local branch is behind `tracking` by.
    pub behind: u32,
    /// Paths with staged changes (a non-empty index column).
    pub staged: Vec<String>,
    /// Paths with unstaged changes to tracked files (a non-empty worktree
    /// column, excluding untracked files).
    pub unstaged: Vec<String>,
    /// Untracked paths (porcelain `??`).
    pub untracked: Vec<String>,
}

impl Changes {
    /// Whether the tree has any staged, unstaged, or untracked change.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        !self.staged.is_empty() || !self.unstaged.is_empty() || !self.untracked.is_empty()
    }

    /// Whether the tree is clean: nothing staged, unstaged, or untracked.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        !self.is_dirty()
    }
}

/// High-level change analysis, scoped to a [`Repository`].
#[derive(Debug)]
pub struct ChangesOps<'a> {
    repo: &'a Repository,
}

impl<'a> ChangesOps<'a> {
    /// Summarize the working tree and index in one `git status` call.
    ///
    /// Runs `git status --porcelain=v1 -b -z`, then classifies each entry by
    /// its `XY` status pair into the staged, unstaged, and untracked lists and
    /// carries through the branch/tracking header with its ahead/behind
    /// counts. A clean tree yields empty lists (but still reports the branch).
    ///
    /// # Errors
    /// Returns an error if the `git status` invocation fails or its output
    /// cannot be parsed.
    pub async fn summary(&self) -> Result<Changes> {
        let out = self
            .repo
            .status()
            .format(StatusFormat::PorcelainV1)
            .branch()
            .null_terminate()
            .execute()
            .await?;
        let status = parse_full_status(&out.stdout_str())?;
        Ok(classify(status))
    }
}

/// Fold a parsed [`Status`] into a typed [`Changes`].
///
/// This is the single classification path: [`summary`](ChangesOps::summary)
/// calls it after running `git status`, and the unit tests exercise it
/// directly with hand-built `Status` values, so the tests pin the exact logic
/// `summary` runs rather than a copy of it.
fn classify(status: Status) -> Changes {
    let mut changes = Changes {
        branch: status.branch,
        tracking: status.tracking,
        ahead: status.ahead,
        behind: status.behind,
        ..Changes::default()
    };

    for entry in status.entries {
        // `??` is reported as Untracked in both columns.
        if entry.index == StatusKind::Untracked || entry.worktree == StatusKind::Untracked {
            changes.untracked.push(entry.path);
            continue;
        }
        if entry.index != StatusKind::Unmodified {
            changes.staged.push(entry.path.clone());
        }
        if entry.worktree != StatusKind::Unmodified {
            changes.unstaged.push(entry.path);
        }
    }

    changes
}

impl Repository {
    /// Typed working-tree and index change analysis.
    #[must_use]
    pub fn changes(&self) -> ChangesOps<'_> {
        ChangesOps { repo: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::StatusEntry;

    fn entry(index: StatusKind, worktree: StatusKind, path: &str) -> StatusEntry {
        StatusEntry {
            index,
            worktree,
            path: path.to_string(),
            original_path: None,
        }
    }

    #[test]
    fn classifies_staged_unstaged_untracked() {
        use StatusKind::{Added, Modified, Unmodified, Untracked};
        let status = Status {
            branch: Some("main".to_string()),
            tracking: Some("origin/main".to_string()),
            ahead: 1,
            behind: 2,
            entries: vec![
                entry(Added, Unmodified, "staged_only.txt"),
                entry(Unmodified, Modified, "unstaged_only.txt"),
                entry(Untracked, Untracked, "untracked.txt"),
            ],
        };
        let c = classify(status);
        assert_eq!(c.branch.as_deref(), Some("main"));
        assert_eq!(c.tracking.as_deref(), Some("origin/main"));
        assert_eq!(c.ahead, 1);
        assert_eq!(c.behind, 2);
        assert_eq!(c.staged, vec!["staged_only.txt"]);
        assert_eq!(c.unstaged, vec!["unstaged_only.txt"]);
        assert_eq!(c.untracked, vec!["untracked.txt"]);
        assert!(c.is_dirty());
    }

    #[test]
    fn index_and_worktree_change_lands_in_both() {
        use StatusKind::Modified;
        let status = Status {
            entries: vec![entry(Modified, Modified, "both.txt")],
            ..Status::default()
        };
        let c = classify(status);
        assert_eq!(c.staged, vec!["both.txt"]);
        assert_eq!(c.unstaged, vec!["both.txt"]);
    }

    #[test]
    fn clean_tree_is_not_dirty() {
        let c = classify(Status {
            branch: Some("main".to_string()),
            ..Status::default()
        });
        assert!(c.is_clean());
        assert!(!c.is_dirty());
        assert_eq!(c.branch.as_deref(), Some("main"));
    }
}
