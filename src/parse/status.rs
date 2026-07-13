//! Parser for `git status --porcelain=v1 -z`.
//!
//! Porcelain v1 is stable across git versions. Each entry is two status
//! characters (`XY`) followed by a space, the path, and a NUL terminator.
//! A rename or copy (signalled in the index column `X`) adds one more
//! NUL-terminated field: the entry's path is the new (destination) path and
//! the extra field is the original path.

use crate::error::{Error, Result};

/// Parsed result of `git status --porcelain=v1 -b -z`: the branch/tracking
/// header plus the per-entry changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Status {
    /// Current branch name. `None` for a detached `HEAD`.
    pub branch: Option<String>,
    /// Upstream tracking ref (e.g. `origin/main`), if one is configured.
    pub tracking: Option<String>,
    /// Commits the local branch is ahead of `tracking` by.
    pub ahead: u32,
    /// Commits the local branch is behind `tracking` by.
    pub behind: u32,
    /// Per-file entries.
    pub entries: Vec<StatusEntry>,
}

/// A single parsed entry from `git status --porcelain=v1 -z`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatusEntry {
    /// Status of the index (the "X" field of `XY`).
    pub index: StatusKind,
    /// Status of the working tree (the "Y" field of `XY`).
    pub worktree: StatusKind,
    /// Affected path (post-rename, if applicable).
    pub path: String,
    /// Original path for renames/copies, if present.
    pub original_path: Option<String>,
}

/// Classification of a status character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StatusKind {
    /// Unmodified (`' '`).
    Unmodified,
    /// Modified (`M`).
    Modified,
    /// Added (`A`).
    Added,
    /// Deleted (`D`).
    Deleted,
    /// Renamed (`R`).
    Renamed,
    /// Copied (`C`).
    Copied,
    /// Unmerged (`U`).
    Unmerged,
    /// Untracked (`?`).
    Untracked,
    /// Ignored (`!`).
    Ignored,
    /// Type-changed (`T`).
    TypeChanged,
    /// Some other character not recognized.
    Other(char),
}

impl From<char> for StatusKind {
    fn from(c: char) -> Self {
        match c {
            ' ' => Self::Unmodified,
            'M' => Self::Modified,
            'A' => Self::Added,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            'U' => Self::Unmerged,
            '?' => Self::Untracked,
            '!' => Self::Ignored,
            'T' => Self::TypeChanged,
            c => Self::Other(c),
        }
    }
}

/// Parse the output of `git status --porcelain=v1 -z`.
///
/// # Errors
/// Returns [`Error::ParseError`] if an entry is malformed.
///
/// # Example
/// ```
/// use git_spawn::parse::{parse_status, StatusKind};
/// // Three entries: modified index+worktree, added, untracked.
/// let input = "MM a.txt\0A  b.txt\0?? c.txt\0";
/// let entries = parse_status(input).unwrap();
/// assert_eq!(entries.len(), 3);
/// assert_eq!(entries[0].index, StatusKind::Modified);
/// assert_eq!(entries[0].worktree, StatusKind::Modified);
/// assert_eq!(entries[2].index, StatusKind::Untracked);
/// ```
pub fn parse_status(input: &str) -> Result<Vec<StatusEntry>> {
    let mut iter = input.split('\0').peekable();
    parse_entries(&mut iter)
}

/// Parse the output of `git status --porcelain=v1 -b -z`, splitting the
/// leading `## ` branch header from the per-entry records.
///
/// # Errors
/// Returns [`Error::ParseError`] if an entry is malformed.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_full_status;
///
/// let input = "## main...origin/main [ahead 1, behind 2]\0 M a.txt\0";
/// let status = parse_full_status(input).unwrap();
/// assert_eq!(status.branch.as_deref(), Some("main"));
/// assert_eq!(status.tracking.as_deref(), Some("origin/main"));
/// assert_eq!(status.ahead, 1);
/// assert_eq!(status.behind, 2);
/// assert_eq!(status.entries.len(), 1);
/// ```
pub fn parse_full_status(input: &str) -> Result<Status> {
    let mut iter = input.split('\0').peekable();
    let (branch, tracking, ahead, behind) = match iter.peek() {
        Some(first) if first.starts_with("##") => {
            let header = iter.next().expect("peeked Some");
            parse_branch_header(header)
        }
        _ => (None, None, 0, 0),
    };
    let entries = parse_entries(&mut iter)?;
    Ok(Status {
        branch,
        tracking,
        ahead,
        behind,
        entries,
    })
}

/// Parse a `## ...` branch header record into `(branch, tracking, ahead, behind)`.
///
/// Handles the no-upstream (`## main`), tracking (`## main...origin/main`),
/// ahead/behind (`## main...origin/main [ahead 1, behind 2]`), no-commits
/// (`## No commits yet on main`), and detached-head (`## HEAD (no branch)`)
/// forms. The detached-head form has no branch name to report, so it yields
/// `branch = None`; the no-commits form still has one, so it is preserved.
fn parse_branch_header(header: &str) -> (Option<String>, Option<String>, u32, u32) {
    let rest = header.trim_start_matches("##").trim();

    if rest == "HEAD (no branch)" {
        return (None, None, 0, 0);
    }

    const NO_COMMITS_PREFIX: &str = "No commits yet on ";
    if let Some(branch) = rest.strip_prefix(NO_COMMITS_PREFIX) {
        return (Some(branch.to_string()), None, 0, 0);
    }

    let (main_part, bracket_part) = match rest.find(" [") {
        Some(idx) => (&rest[..idx], Some(rest[idx + 2..].trim_end_matches(']'))),
        None => (rest, None),
    };

    let (branch, tracking) = match main_part.find("...") {
        Some(pos) => (
            Some(main_part[..pos].to_string()),
            Some(main_part[pos + 3..].to_string()),
        ),
        None => (Some(main_part.to_string()), None),
    };

    let mut ahead = 0;
    let mut behind = 0;
    if let Some(bracket) = bracket_part {
        for part in bracket.split(',') {
            let part = part.trim();
            if let Some(n) = part.strip_prefix("ahead ") {
                ahead = n.trim().parse().unwrap_or(0);
            } else if let Some(n) = part.strip_prefix("behind ") {
                behind = n.trim().parse().unwrap_or(0);
            }
        }
    }

    (branch, tracking, ahead, behind)
}

/// Shared entry-parsing loop used by both [`parse_status`] and
/// [`parse_full_status`], operating on an already-positioned iterator (i.e.
/// past any `## ` header record).
fn parse_entries(
    iter: &mut std::iter::Peekable<std::str::Split<'_, char>>,
) -> Result<Vec<StatusEntry>> {
    let mut out = Vec::new();
    while let Some(record) = iter.next() {
        if record.is_empty() {
            continue;
        }
        let mut chars = record.chars();
        let x = chars
            .next()
            .ok_or_else(|| Error::parse_error("status entry missing X field"))?;
        let y = chars
            .next()
            .ok_or_else(|| Error::parse_error("status entry missing Y field"))?;
        // Expect a single space separator, then the path.
        if chars.next() != Some(' ') {
            return Err(Error::parse_error(
                "status entry missing space after XY field",
            ));
        }
        let path: String = chars.collect();
        let kind_x = StatusKind::from(x);
        let kind_y = StatusKind::from(y);
        // In porcelain v1 -z, a rename or copy is signalled in the index
        // column (X) and is followed by exactly one extra NUL-terminated
        // field: the original path. The path on the XY record itself is the
        // new (destination) path. git does not run rename detection against
        // the worktree, so the worktree column (Y) never carries R/C in v1 --
        // only X drives the extra read. Keying on Y as well would consume the
        // following entry's record as a phantom original path.
        let original = if matches!(kind_x, StatusKind::Renamed | StatusKind::Copied) {
            iter.next().map(str::to_string)
        } else {
            None
        };
        out.push(StatusEntry {
            index: kind_x,
            worktree: kind_y,
            path,
            original_path: original,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_entries() {
        let input = "MM a.txt\0A  b.txt\0?? c.txt\0";
        let entries = parse_status(input).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "a.txt");
        assert_eq!(entries[1].index, StatusKind::Added);
        assert_eq!(entries[1].worktree, StatusKind::Unmodified);
        assert_eq!(entries[2].index, StatusKind::Untracked);
    }

    #[test]
    fn parses_rename_with_original() {
        let input = "R  new.txt\0old.txt\0";
        let entries = parse_status(input).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].index, StatusKind::Renamed);
        assert_eq!(entries[0].path, "new.txt");
        assert_eq!(entries[0].original_path.as_deref(), Some("old.txt"));
    }

    #[test]
    fn parses_rename_with_worktree_modification() {
        // Real git emits `RM <new>\0<old>\0` when a staged rename is also
        // modified in the worktree. The original-path field must be consumed
        // so the following entry parses cleanly.
        let input = "RM new.txt\0old.txt\0MM other.txt\0";
        let entries = parse_status(input).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, StatusKind::Renamed);
        assert_eq!(entries[0].worktree, StatusKind::Modified);
        assert_eq!(entries[0].path, "new.txt");
        assert_eq!(entries[0].original_path.as_deref(), Some("old.txt"));
        assert_eq!(entries[1].path, "other.txt");
        assert_eq!(entries[1].original_path, None);
    }

    #[test]
    fn parses_copy_with_original() {
        let input = "C  copy.txt\0src.txt\0";
        let entries = parse_status(input).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].index, StatusKind::Copied);
        assert_eq!(entries[0].path, "copy.txt");
        assert_eq!(entries[0].original_path.as_deref(), Some("src.txt"));
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_status("").unwrap().is_empty());
    }

    #[test]
    fn malformed_missing_space_errors() {
        let input = "MMa.txt\0";
        assert!(parse_status(input).is_err());
    }

    #[test]
    fn full_status_no_upstream() {
        let input = "## main\0 M a.txt\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.tracking, None);
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert_eq!(status.entries.len(), 1);
    }

    #[test]
    fn full_status_upstream_only() {
        let input = "## main...origin/main\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.tracking.as_deref(), Some("origin/main"));
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert!(status.entries.is_empty());
    }

    #[test]
    fn full_status_ahead_only() {
        let input = "## main...origin/main [ahead 3]\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.tracking.as_deref(), Some("origin/main"));
        assert_eq!(status.ahead, 3);
        assert_eq!(status.behind, 0);
    }

    #[test]
    fn full_status_behind_only() {
        let input = "## main...origin/main [behind 5]\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 5);
    }

    #[test]
    fn full_status_ahead_and_behind() {
        let input = "## main...origin/main [ahead 1, behind 2]\0 M a.txt\0?? b.txt\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.tracking.as_deref(), Some("origin/main"));
        assert_eq!(status.ahead, 1);
        assert_eq!(status.behind, 2);
        assert_eq!(status.entries.len(), 2);
    }

    #[test]
    fn full_status_no_commits_yet() {
        let input = "## No commits yet on main\0?? a.txt\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.tracking, None);
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert_eq!(status.entries.len(), 1);
    }

    #[test]
    fn full_status_detached_head() {
        let input = "## HEAD (no branch)\0 M a.txt\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.branch, None);
        assert_eq!(status.tracking, None);
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
        assert_eq!(status.entries.len(), 1);
    }

    #[test]
    fn full_status_no_header_still_parses_entries() {
        let input = " M a.txt\0";
        let status = parse_full_status(input).unwrap();
        assert_eq!(status.branch, None);
        assert_eq!(status.entries.len(), 1);
    }
}
