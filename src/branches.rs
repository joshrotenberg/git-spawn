//! Typed listing and bulk operations on local branches.
//!
//! Reached through [`Repository::branches`], which returns a [`BranchOps`]
//! handle:
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // All local branches with upstream / ahead-behind info.
//! for b in repo.branches().list().await? {
//!     println!("{}{}  {}  ({})",
//!         if b.current { "* " } else { "  " },
//!         b.name,
//!         b.subject.as_deref().unwrap_or(""),
//!         b.upstream.as_deref().unwrap_or("no upstream"),
//!     );
//! }
//!
//! // Delete branches merged into main.
//! let deleted = repo.branches().delete_merged("main").await?;
//! println!("removed {} merged branch(es)", deleted.len());
//! # Ok(())
//! # }
//! ```
//!
//! Listing is implemented with `git for-each-ref refs/heads/` and a fixed,
//! NUL-delimited format string, so the parser is deterministic across git
//! versions and locales.

use crate::command::GitCommand;
use crate::command::branch::BranchCommand;
use crate::command::for_each_ref::ForEachRefCommand;
use crate::error::Result;
use crate::repo::Repository;

/// One local branch with tracking info.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Branch {
    /// Short branch name (e.g. `"main"`).
    pub name: String,
    /// `true` when this is the currently-checked-out branch.
    pub current: bool,
    /// Configured upstream in `remote/branch` form, if any.
    pub upstream: Option<String>,
    /// Commits this branch is ahead of its upstream.
    pub ahead: u32,
    /// Commits this branch is behind its upstream.
    pub behind: u32,
    /// `true` when the configured upstream no longer exists (`[gone]`).
    pub upstream_gone: bool,
    /// Short SHA of the branch tip.
    pub head: String,
    /// Subject line of the tip commit, if non-empty.
    pub subject: Option<String>,
}

/// Operations on local branches, scoped to a [`Repository`].
///
/// Obtained via [`Repository::branches`]. The handle borrows the repository
/// for the duration of one chained call — there is no shared state.
#[derive(Debug)]
pub struct BranchOps<'a> {
    repo: &'a Repository,
}

impl<'a> BranchOps<'a> {
    /// List every local branch.
    pub async fn list(&self) -> Result<Vec<Branch>> {
        self.list_inner(None).await
    }

    /// List branches whose ref path matches `pattern` (a `fnmatch`-style glob
    /// applied to the full ref, e.g. `"refs/heads/feature/*"`). For short-name
    /// matching, pass `"refs/heads/<glob>"`.
    pub async fn list_matching(&self, pattern: impl Into<String>) -> Result<Vec<Branch>> {
        self.list_inner(Some(pattern.into())).await
    }

    /// Delete every local branch fully merged into `into` (other than `into`
    /// itself and the current branch). Returns the deleted branch names.
    pub async fn delete_merged(&self, into: impl AsRef<str>) -> Result<Vec<String>> {
        let into = into.as_ref();
        let current = self.list().await?.into_iter().find(|b| b.current);
        let current_name = current.as_ref().map(|b| b.name.as_str());

        let mut cmd = ForEachRefCommand::new();
        cmd.pattern("refs/heads/")
            .format("%(refname:short)".to_string())
            .merged(into.to_string());
        cmd.current_dir(self.repo.path());
        let out = cmd.execute().await?;

        let mut deleted = Vec::new();
        for name in out.stdout.lines() {
            if name.is_empty() || name == into || Some(name) == current_name {
                continue;
            }
            let mut del = BranchCommand::new();
            del.delete(name);
            del.current_dir(self.repo.path());
            del.execute().await?;
            deleted.push(name.to_string());
        }
        Ok(deleted)
    }

    /// Rename a branch (`git branch -m <from> <to>`).
    pub async fn rename(&self, from: impl Into<String>, to: impl Into<String>) -> Result<()> {
        let mut cmd = BranchCommand::new();
        cmd.rename(from, to);
        cmd.current_dir(self.repo.path());
        cmd.execute().await?;
        Ok(())
    }

    async fn list_inner(&self, pattern: Option<String>) -> Result<Vec<Branch>> {
        let mut cmd = ForEachRefCommand::new();
        cmd.format(FORMAT.to_string())
            .pattern(pattern.unwrap_or_else(|| "refs/heads/".to_string()));
        cmd.current_dir(self.repo.path());
        let out = cmd.execute().await?;
        parse_branches(&out.stdout)
    }
}

impl Repository {
    /// Operations on local branches.
    #[must_use]
    pub fn branches(&self) -> BranchOps<'_> {
        BranchOps { repo: self }
    }
}

/// NUL-delimited format for one record. Field order matches [`parse_branches`].
const FORMAT: &str = concat!(
    "%(refname:short)",
    "%00",
    "%(HEAD)",
    "%00",
    "%(upstream:short)",
    "%00",
    "%(upstream:track)",
    "%00",
    "%(objectname:short)",
    "%00",
    "%(contents:subject)",
);

fn parse_branches(stdout: &str) -> Result<Vec<Branch>> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\0').collect();
        if fields.len() < 6 {
            return Err(crate::error::Error::parse_error(format!(
                "branch record has {} fields, expected 6: {line:?}",
                fields.len()
            )));
        }
        let (ahead, behind, gone) = parse_track(fields[3]);
        out.push(Branch {
            name: fields[0].to_string(),
            current: fields[1] == "*",
            upstream: if fields[2].is_empty() {
                None
            } else {
                Some(fields[2].to_string())
            },
            ahead,
            behind,
            upstream_gone: gone,
            head: fields[4].to_string(),
            subject: if fields[5].is_empty() {
                None
            } else {
                Some(fields[5].to_string())
            },
        });
    }
    Ok(out)
}

fn parse_track(s: &str) -> (u32, u32, bool) {
    // Possible shapes: "", "[gone]", "[ahead 1]", "[behind 2]", "[ahead 1, behind 2]"
    let inside = s.trim().trim_start_matches('[').trim_end_matches(']');
    if inside.is_empty() {
        return (0, 0, false);
    }
    if inside == "gone" {
        return (0, 0, true);
    }
    let mut ahead = 0;
    let mut behind = 0;
    for part in inside.split(',') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("ahead ") {
            ahead = n.parse().unwrap_or(0);
        } else if let Some(n) = part.strip_prefix("behind ") {
            behind = n.parse().unwrap_or(0);
        }
    }
    (ahead, behind, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_track_field() {
        assert_eq!(parse_track(""), (0, 0, false));
        assert_eq!(parse_track("[gone]"), (0, 0, true));
        assert_eq!(parse_track("[ahead 3]"), (3, 0, false));
        assert_eq!(parse_track("[behind 2]"), (0, 2, false));
        assert_eq!(parse_track("[ahead 1, behind 4]"), (1, 4, false));
    }

    #[test]
    fn parses_branch_records() {
        let line1 = "main\0*\0origin/main\0[ahead 1]\0abc1234\0fix: things";
        let line2 = "feature/x\0 \0\0\0def5678\0";
        let input = format!("{line1}\n{line2}\n");
        let branches = parse_branches(&input).unwrap();
        assert_eq!(branches.len(), 2);

        assert_eq!(branches[0].name, "main");
        assert!(branches[0].current);
        assert_eq!(branches[0].upstream.as_deref(), Some("origin/main"));
        assert_eq!(branches[0].ahead, 1);
        assert_eq!(branches[0].behind, 0);
        assert!(!branches[0].upstream_gone);
        assert_eq!(branches[0].head, "abc1234");
        assert_eq!(branches[0].subject.as_deref(), Some("fix: things"));

        assert_eq!(branches[1].name, "feature/x");
        assert!(!branches[1].current);
        assert!(branches[1].upstream.is_none());
        assert!(branches[1].subject.is_none());
        assert_eq!(branches[1].head, "def5678");
    }

    #[test]
    fn malformed_record_errors() {
        let input = "only\0three\0fields\n";
        assert!(parse_branches(input).is_err());
    }
}
