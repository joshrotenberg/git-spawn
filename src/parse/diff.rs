//! Parser for `git diff --name-status -z`.
//!
//! Each entry is a single status character followed by a tab and the path,
//! all NUL-terminated. Renames/copies include a numeric similarity index
//! (e.g. `R100`) and a second path following another NUL.

use crate::error::{Error, Result};

/// One parsed entry from `git diff --name-status -z`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiffEntry {
    /// Kind of change.
    pub kind: DiffKind,
    /// Affected path (post-rename, if applicable).
    pub path: String,
    /// Original path for renames/copies.
    pub original_path: Option<String>,
    /// Similarity index for renames/copies (e.g. 100 means identical).
    pub similarity: Option<u32>,
}

/// Classification of a diff entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiffKind {
    /// Added (`A`).
    Added,
    /// Deleted (`D`).
    Deleted,
    /// Modified (`M`).
    Modified,
    /// Renamed (`R`).
    Renamed,
    /// Copied (`C`).
    Copied,
    /// Type changed (`T`).
    TypeChanged,
    /// Unmerged (`U`).
    Unmerged,
    /// Unknown (`X` or anything else).
    Other(char),
}

impl From<char> for DiffKind {
    fn from(c: char) -> Self {
        match c {
            'A' => Self::Added,
            'D' => Self::Deleted,
            'M' => Self::Modified,
            'R' => Self::Renamed,
            'C' => Self::Copied,
            'T' => Self::TypeChanged,
            'U' => Self::Unmerged,
            c => Self::Other(c),
        }
    }
}

/// Parse the output of `git diff --name-status -z`.
///
/// # Errors
/// Returns [`Error::ParseError`] if an entry is malformed.
///
/// # Example
/// ```
/// use git_spawn::parse::{parse_diff_name_status, DiffKind};
/// let input = "M\0foo.txt\0A\0bar.txt\0R100\0old.rs\0new.rs\0";
/// let entries = parse_diff_name_status(input).unwrap();
/// assert_eq!(entries.len(), 3);
/// assert_eq!(entries[0].kind, DiffKind::Modified);
/// assert_eq!(entries[2].kind, DiffKind::Renamed);
/// assert_eq!(entries[2].similarity, Some(100));
/// assert_eq!(entries[2].original_path.as_deref(), Some("old.rs"));
/// assert_eq!(entries[2].path, "new.rs");
/// ```
pub fn parse_diff_name_status(input: &str) -> Result<Vec<DiffEntry>> {
    let mut out = Vec::new();
    let mut iter = input.split('\0');
    while let Some(status) = iter.next() {
        if status.is_empty() {
            continue;
        }
        let mut chars = status.chars();
        let first = chars
            .next()
            .ok_or_else(|| Error::parse_error("diff entry missing status character"))?;
        let kind = DiffKind::from(first);
        let similarity: Option<u32> = {
            let rest: String = chars.collect();
            if rest.is_empty() {
                None
            } else {
                rest.parse().ok()
            }
        };
        let is_rename_or_copy = matches!(kind, DiffKind::Renamed | DiffKind::Copied);
        let (original, path) = if is_rename_or_copy {
            let orig = iter
                .next()
                .ok_or_else(|| Error::parse_error("rename/copy missing original path"))?;
            let new = iter
                .next()
                .ok_or_else(|| Error::parse_error("rename/copy missing new path"))?;
            (Some(orig.to_string()), new.to_string())
        } else {
            let path = iter
                .next()
                .ok_or_else(|| Error::parse_error("diff entry missing path"))?;
            (None, path.to_string())
        };
        out.push(DiffEntry {
            kind,
            path,
            original_path: original,
            similarity,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_changes() {
        let input = "M\0foo.txt\0A\0bar.txt\0D\0baz.txt\0";
        let entries = parse_diff_name_status(input).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, "foo.txt");
        assert_eq!(entries[2].kind, DiffKind::Deleted);
    }

    #[test]
    fn rename_with_similarity() {
        let input = "R090\0a.rs\0b.rs\0";
        let entries = parse_diff_name_status(input).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].similarity, Some(90));
        assert_eq!(entries[0].original_path.as_deref(), Some("a.rs"));
        assert_eq!(entries[0].path, "b.rs");
    }

    #[test]
    fn empty_ok() {
        assert!(parse_diff_name_status("").unwrap().is_empty());
    }
}
