//! Parser for `git status --porcelain=v1 -z`.
//!
//! Porcelain v1 is stable across git versions. Each entry is two status
//! characters (`XY`) followed by a space, the path, and a NUL terminator.
//! Renames and copies add a second NUL-terminated path.

use crate::error::{Error, Result};

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
/// use git_wrapper::parse::{parse_status, StatusKind};
/// // Three entries: modified index+worktree, added, untracked.
/// let input = "MM a.txt\0A  b.txt\0?? c.txt\0";
/// let entries = parse_status(input).unwrap();
/// assert_eq!(entries.len(), 3);
/// assert_eq!(entries[0].index, StatusKind::Modified);
/// assert_eq!(entries[0].worktree, StatusKind::Modified);
/// assert_eq!(entries[2].index, StatusKind::Untracked);
/// ```
pub fn parse_status(input: &str) -> Result<Vec<StatusEntry>> {
    let mut out = Vec::new();
    let mut iter = input.split('\0').peekable();
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
        let original = if matches!(kind_x, StatusKind::Renamed | StatusKind::Copied)
            || matches!(kind_y, StatusKind::Renamed | StatusKind::Copied)
        {
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
    fn empty_input_yields_no_entries() {
        assert!(parse_status("").unwrap().is_empty());
    }

    #[test]
    fn malformed_missing_space_errors() {
        let input = "MMa.txt\0";
        assert!(parse_status(input).is_err());
    }
}
