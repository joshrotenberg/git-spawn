//! Repository overview in a single call.
//!
//! [`RepoInfo`] folds together the bits most automation reaches for first —
//! current branch, upstream tracking, default branch, dirty state, ahead/behind
//! counts — and is produced by [`Repository::info`].
//!
//! Behind a single call we run `git status --porcelain=v2 --branch` (which
//! emits a stable header with branch / upstream / ab counts plus per-file
//! entries) and `git symbolic-ref refs/remotes/origin/HEAD` for the default
//! branch. The default-branch lookup fails silently when there is no `origin`
//! remote yet — the field stays `None` rather than surfacing an error.
//!
//! # Example
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//! let info = repo.info().await?;
//!
//! if info.dirty {
//!     eprintln!("uncommitted changes on {}", info.branch.as_deref().unwrap_or("(detached)"));
//! }
//! if info.behind > 0 {
//!     eprintln!("{} commits behind {}", info.behind, info.upstream.as_deref().unwrap_or("upstream"));
//! }
//! # Ok(())
//! # }
//! ```

use crate::command::GitCommand;
use crate::command::status::StatusFormat;
use crate::command::symbolic_ref::SymbolicRefCommand;
use crate::error::Result;
use crate::repo::Repository;

/// Snapshot of a repository's state.
///
/// Fields are populated independently — a missing upstream leaves `upstream`,
/// `ahead`, and `behind` at their defaults (`None`, `0`, `0`); a missing remote
/// leaves `default_branch` at `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RepoInfo {
    /// Current branch, or `None` when `HEAD` is detached.
    pub branch: Option<String>,
    /// Configured upstream in `remote/branch` form (e.g. `"origin/main"`).
    pub upstream: Option<String>,
    /// Default branch as advertised by `refs/remotes/origin/HEAD`
    /// (short form, e.g. `"main"`). `None` when no `origin` remote is
    /// configured or the symbolic ref is missing.
    pub default_branch: Option<String>,
    /// `true` when the working tree or index has any pending changes
    /// (modified, staged, untracked, etc.).
    pub dirty: bool,
    /// Commits the current branch is ahead of its upstream.
    pub ahead: u32,
    /// Commits the current branch is behind its upstream.
    pub behind: u32,
}

impl Repository {
    /// Collect a [`RepoInfo`] snapshot in a single call.
    ///
    /// Runs `git status --porcelain=v2 --branch` plus one `symbolic-ref` lookup
    /// for the default branch. See the [module docs](self) for details and
    /// caveats.
    pub async fn info(&self) -> Result<RepoInfo> {
        let status_out = self
            .status()
            .format(StatusFormat::PorcelainV2)
            .branch()
            .execute()
            .await?;

        let mut info = parse_porcelain_v2(&status_out.stdout_str());

        let mut sym = SymbolicRefCommand::read("refs/remotes/origin/HEAD");
        sym.short().current_dir(self.path());
        if let Ok(target) = sym.execute().await {
            let short = target
                .strip_prefix("origin/")
                .map_or_else(|| target.clone(), str::to_string);
            if !short.is_empty() {
                info.default_branch = Some(short);
            }
        }

        Ok(info)
    }
}

fn parse_porcelain_v2(stdout: &str) -> RepoInfo {
    let mut info = RepoInfo::default();
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            if rest != "(detached)" {
                info.branch = Some(rest.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("# branch.upstream ") {
            info.upstream = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            let mut parts = rest.split_whitespace();
            if let Some(a) = parts.next() {
                info.ahead = a.trim_start_matches('+').parse().unwrap_or(0);
            }
            if let Some(b) = parts.next() {
                info.behind = b.trim_start_matches('-').parse().unwrap_or(0);
            }
        } else if !line.is_empty() && !line.starts_with('#') {
            info.dirty = true;
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_repo_with_upstream() {
        let input = "\
# branch.oid abc123
# branch.head main
# branch.upstream origin/main
# branch.ab +0 -0
";
        let info = parse_porcelain_v2(input);
        assert_eq!(info.branch.as_deref(), Some("main"));
        assert_eq!(info.upstream.as_deref(), Some("origin/main"));
        assert_eq!(info.ahead, 0);
        assert_eq!(info.behind, 0);
        assert!(!info.dirty);
    }

    #[test]
    fn parses_dirty_with_ahead_behind() {
        let input = "\
# branch.oid abc123
# branch.head feature
# branch.upstream origin/feature
# branch.ab +3 -1
1 .M N... 100644 100644 100644 aaa bbb hello.txt
? new.txt
";
        let info = parse_porcelain_v2(input);
        assert_eq!(info.branch.as_deref(), Some("feature"));
        assert_eq!(info.ahead, 3);
        assert_eq!(info.behind, 1);
        assert!(info.dirty);
    }

    #[test]
    fn parses_detached_head() {
        let input = "\
# branch.oid abc123
# branch.head (detached)
";
        let info = parse_porcelain_v2(input);
        assert!(info.branch.is_none());
        assert!(info.upstream.is_none());
        assert!(!info.dirty);
    }

    #[test]
    fn parses_no_upstream() {
        let input = "\
# branch.oid abc123
# branch.head main
";
        let info = parse_porcelain_v2(input);
        assert_eq!(info.branch.as_deref(), Some("main"));
        assert!(info.upstream.is_none());
        assert_eq!(info.ahead, 0);
        assert_eq!(info.behind, 0);
    }
}
