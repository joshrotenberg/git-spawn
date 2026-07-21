//! Typed access to merge/rebase conflicts.
//!
//! Reached through [`Repository::conflicts`], which returns a [`ConflictOps`]
//! handle. [`list`](ConflictOps::list) runs `git status --porcelain=v1 -z`,
//! keeps the unmerged entries, and classifies each one into a typed
//! [`Conflict`]; [`resolve`](ConflictOps::resolve) stages a path
//! (`git add <path>`) to mark it resolved.
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Inspect the paths left conflicted by an in-progress merge or rebase.
//! for conflict in repo.conflicts().list().await? {
//!     println!("{} ({:?})", conflict.path, conflict.kind);
//! }
//!
//! // After editing `foo.txt` to resolve it, stage the result.
//! repo.conflicts().resolve("foo.txt").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Classification comes from the porcelain `XY` status pair, so it reflects
//! whichever side(s) touched the path (see [`ConflictKind`]). `resolve` stages
//! the current working-tree state; to accept a deletion instead, reach for a
//! raw `git rm` via [`Repository::rm`].

use crate::command::GitCommand;
use crate::command::status::StatusFormat;
use crate::error::Result;
use crate::parse::{StatusKind, parse_status};
use crate::repo::Repository;

/// How a path is conflicted, read from the porcelain `XY` status pair.
///
/// "Us" is the branch being merged into (the current `HEAD`); "them" is the
/// branch being merged in. The variants mirror git's unmerged status codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ConflictKind {
    /// Both sides modified the path (`UU`).
    BothModified,
    /// Both sides added the path (`AA`).
    BothAdded,
    /// Both sides deleted the path (`DD`).
    BothDeleted,
    /// We added the path; they left it unmerged (`AU`).
    AddedByUs,
    /// They added the path; we left it unmerged (`UA`).
    AddedByThem,
    /// We deleted the path; they modified it (`DU`).
    DeletedByUs,
    /// They deleted the path; we modified it (`UD`).
    DeletedByThem,
}

impl ConflictKind {
    /// Classify an unmerged entry from its `(index, worktree)` status pair.
    ///
    /// Returns `None` for any pair that is not one of git's seven unmerged
    /// combinations, so this doubles as the filter that keeps only conflicted
    /// entries.
    fn from_pair(index: StatusKind, worktree: StatusKind) -> Option<Self> {
        use StatusKind::{Added, Deleted, Unmerged};
        Some(match (index, worktree) {
            (Unmerged, Unmerged) => Self::BothModified,
            (Added, Added) => Self::BothAdded,
            (Deleted, Deleted) => Self::BothDeleted,
            (Added, Unmerged) => Self::AddedByUs,
            (Unmerged, Added) => Self::AddedByThem,
            (Deleted, Unmerged) => Self::DeletedByUs,
            (Unmerged, Deleted) => Self::DeletedByThem,
            _ => return None,
        })
    }
}

/// A single conflicted path in an in-progress merge or rebase.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Conflict {
    /// The conflicted path, relative to the repository root.
    pub path: String,
    /// Which side(s) touched the path.
    pub kind: ConflictKind,
}

/// High-level conflict inspection and resolution, scoped to a [`Repository`].
#[derive(Debug)]
pub struct ConflictOps<'a> {
    repo: &'a Repository,
}

impl<'a> ConflictOps<'a> {
    /// List the paths left conflicted by an in-progress merge or rebase.
    ///
    /// Runs `git status --porcelain=v1 -z` and keeps only the unmerged
    /// entries, classifying each by its `XY` code. A clean tree (or one with
    /// no conflicts) yields an empty vector.
    ///
    /// # Errors
    /// Returns an error if the `git status` invocation fails or its output
    /// cannot be parsed.
    pub async fn list(&self) -> Result<Vec<Conflict>> {
        let out = self
            .repo
            .status()
            .format(StatusFormat::PorcelainV1)
            .null_terminate()
            .execute()
            .await?;
        let entries = parse_status(&out.stdout_str())?;
        Ok(entries
            .into_iter()
            .filter_map(|e| {
                ConflictKind::from_pair(e.index, e.worktree)
                    .map(|kind| Conflict { path: e.path, kind })
            })
            .collect())
    }

    /// Mark `path` resolved by staging its current working-tree state.
    ///
    /// Equivalent to `git add <path>`. Edit the file to your resolved content
    /// first; this only records that the conflict is settled. To resolve by
    /// accepting a deletion, use `git rm` instead.
    ///
    /// # Errors
    /// Returns an error if the `git add` invocation fails.
    pub async fn resolve(&self, path: impl Into<String>) -> Result<()> {
        let mut cmd = self.repo.add();
        cmd.path(path);
        cmd.execute().await?;
        Ok(())
    }
}

impl Repository {
    /// Typed access to merge/rebase conflicts.
    #[must_use]
    pub fn conflicts(&self) -> ConflictOps<'_> {
        ConflictOps { repo: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_every_unmerged_pair() {
        use StatusKind::{Added, Deleted, Unmerged};
        assert_eq!(
            ConflictKind::from_pair(Unmerged, Unmerged),
            Some(ConflictKind::BothModified)
        );
        assert_eq!(
            ConflictKind::from_pair(Added, Added),
            Some(ConflictKind::BothAdded)
        );
        assert_eq!(
            ConflictKind::from_pair(Deleted, Deleted),
            Some(ConflictKind::BothDeleted)
        );
        assert_eq!(
            ConflictKind::from_pair(Added, Unmerged),
            Some(ConflictKind::AddedByUs)
        );
        assert_eq!(
            ConflictKind::from_pair(Unmerged, Added),
            Some(ConflictKind::AddedByThem)
        );
        assert_eq!(
            ConflictKind::from_pair(Deleted, Unmerged),
            Some(ConflictKind::DeletedByUs)
        );
        assert_eq!(
            ConflictKind::from_pair(Unmerged, Deleted),
            Some(ConflictKind::DeletedByThem)
        );
    }

    #[test]
    fn ignores_non_conflict_pairs() {
        use StatusKind::{Added, Modified, Unmodified};
        // A staged addition, a worktree modification, and a clean entry are
        // not conflicts.
        assert_eq!(ConflictKind::from_pair(Added, Unmodified), None);
        assert_eq!(ConflictKind::from_pair(Unmodified, Modified), None);
        assert_eq!(ConflictKind::from_pair(Modified, Modified), None);
    }
}
