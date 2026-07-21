//! Parsers for `git diff` output: name-status, numstat, and stat.
//!
//! `git diff --name-status -z` entries are a single status character
//! followed by a tab and the path, all NUL-terminated. Renames/copies
//! include a numeric similarity index (e.g. `R100`) and a second path
//! following another NUL.
//!
//! `git diff --numstat -z` and `git diff --stat` are parsed into a shared
//! [`Diff`] aggregate of per-file [`DiffFile`] insertion/deletion counts plus
//! repo-level totals. `--numstat -z` gives exact per-file counts; `--stat`'s
//! per-file counts are derived from its `+`/`-` graph, which git scales down
//! for large diffs, so prefer `--numstat -z` when exact per-file counts
//! matter. The repo-level totals parsed from `--stat`'s summary line are
//! exact either way.

use crate::error::{Error, Result};

/// Aggregate result of parsing `git diff --numstat -z` or `git diff --stat`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Diff {
    /// Per-file change counts.
    pub files: Vec<DiffFile>,
    /// Total inserted lines across all files.
    pub total_insertions: u32,
    /// Total deleted lines across all files.
    pub total_deletions: u32,
    /// Full unparsed stdout.
    pub raw: String,
}

/// A single file's change counts within a [`Diff`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiffFile {
    /// Affected path (post-rename, if applicable).
    pub path: String,
    /// Inserted lines.
    pub insertions: u32,
    /// Deleted lines.
    pub deletions: u32,
    /// Whether the file is binary (no line counts available).
    pub binary: bool,
}

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

/// Parse the output of `git diff --numstat -z`.
///
/// Each record is `insertions\tdeletions\tpath`, NUL-terminated. Binary
/// files report `-\t-\t` in place of counts. A rename/copy leaves the path
/// field empty and follows with two more NUL-terminated fields, the
/// original and new path (mirroring [`parse_diff_name_status`]); only the
/// new path is kept, since [`DiffFile`] has no original-path field.
///
/// # Errors
/// Returns [`Error::ParseError`] if a record is malformed.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_diff_numstat;
/// let input = "3\t1\tfoo.txt\0-\t-\timg.png\0";
/// let diff = parse_diff_numstat(input).unwrap();
/// assert_eq!(diff.files.len(), 2);
/// assert_eq!(diff.files[0].insertions, 3);
/// assert!(diff.files[1].binary);
/// assert_eq!(diff.total_insertions, 3);
/// assert_eq!(diff.total_deletions, 1);
/// ```
pub fn parse_diff_numstat(input: &str) -> Result<Diff> {
    let mut files = Vec::new();
    let mut total_insertions = 0u32;
    let mut total_deletions = 0u32;
    let mut iter = input.split('\0');
    while let Some(record) = iter.next() {
        if record.is_empty() {
            continue;
        }
        let mut fields = record.splitn(3, '\t');
        let ins_field = fields
            .next()
            .ok_or_else(|| Error::parse_error("numstat entry missing insertions field"))?;
        let del_field = fields
            .next()
            .ok_or_else(|| Error::parse_error("numstat entry missing deletions field"))?;
        let path_field = fields.next().unwrap_or("");

        let binary = ins_field == "-" && del_field == "-";
        let (insertions, deletions) = if binary {
            (0, 0)
        } else {
            let ins: u32 = ins_field
                .parse()
                .map_err(|_| Error::parse_error("invalid numstat insertions count"))?;
            let del: u32 = del_field
                .parse()
                .map_err(|_| Error::parse_error("invalid numstat deletions count"))?;
            (ins, del)
        };

        let path = if path_field.is_empty() {
            iter.next()
                .ok_or_else(|| Error::parse_error("numstat rename missing original path"))?;
            iter.next()
                .ok_or_else(|| Error::parse_error("numstat rename missing new path"))?
                .to_string()
        } else {
            path_field.to_string()
        };

        total_insertions += insertions;
        total_deletions += deletions;
        files.push(DiffFile {
            path,
            insertions,
            deletions,
            binary,
        });
    }
    Ok(Diff {
        files,
        total_insertions,
        total_deletions,
        raw: input.to_string(),
    })
}

/// Parse the output of `git diff --stat`.
///
/// Per-file lines look like `path | N +++---` or `path | Bin N -> M bytes`;
/// a trailing summary line gives repo-level totals (e.g. `2 files changed, 3
/// insertions(+), 1 deletion(-)`). Renamed paths appear as `old => new` or,
/// for changes confined to part of the path, `prefix{old => new}suffix`;
/// both are resolved down to the new path.
///
/// Per-file insertion/deletion counts are derived by counting the `+`/`-`
/// characters in the graph, which git scales down for large diffs — treat
/// them as approximate. The summary line's totals are exact and populate
/// [`Diff::total_insertions`]/[`Diff::total_deletions`].
///
/// # Errors
/// This never returns an error; unrecognized lines are skipped.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_diff_stat;
/// let input = " foo.txt | 3 ++-\n bin.dat | Bin 0 -> 5 bytes\n 2 files changed, 2 insertions(+), 1 deletion(-)\n";
/// let diff = parse_diff_stat(input).unwrap();
/// assert_eq!(diff.files.len(), 2);
/// assert_eq!(diff.files[0].path, "foo.txt");
/// assert!(diff.files[1].binary);
/// assert_eq!(diff.total_insertions, 2);
/// assert_eq!(diff.total_deletions, 1);
/// ```
pub fn parse_diff_stat(input: &str) -> Result<Diff> {
    let mut files = Vec::new();
    let mut total_insertions = 0u32;
    let mut total_deletions = 0u32;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(file) = parse_stat_file_line(line) {
            files.push(file);
        } else if line.contains("changed") {
            let (ins, del) = parse_stat_summary_line(line);
            total_insertions = ins;
            total_deletions = del;
        }
    }

    Ok(Diff {
        files,
        total_insertions,
        total_deletions,
        raw: input.to_string(),
    })
}

/// Parse a single `--stat` per-file line, or `None` if it isn't one (i.e.
/// has no ` | ` separator, as the summary line doesn't).
fn parse_stat_file_line(line: &str) -> Option<DiffFile> {
    let (raw_path, rest) = line.split_once(" | ")?;
    let path = resolve_stat_path(raw_path);
    let rest = rest.trim();

    if rest.starts_with("Bin") {
        return Some(DiffFile {
            path,
            insertions: 0,
            deletions: 0,
            binary: true,
        });
    }

    let insertions = rest.chars().filter(|&c| c == '+').count() as u32;
    let deletions = rest.chars().filter(|&c| c == '-').count() as u32;
    Some(DiffFile {
        path,
        insertions,
        deletions,
        binary: false,
    })
}

/// Resolve a `--stat` path column down to the new path, expanding rename
/// notation: plain `old => new` and the common-prefix/suffix abbreviated
/// form `prefix{old => new}suffix`.
fn resolve_stat_path(raw: &str) -> String {
    let raw = raw.trim();
    if let Some(brace_start) = raw.find('{') {
        if let Some(brace_end) = raw[brace_start..].find('}').map(|i| i + brace_start) {
            let inner = &raw[brace_start + 1..brace_end];
            if let Some(arrow) = inner.find(" => ") {
                let prefix = &raw[..brace_start];
                let suffix = &raw[brace_end + 1..];
                let new_part = &inner[arrow + " => ".len()..];
                return format!("{prefix}{new_part}{suffix}");
            }
        }
    }
    if let Some(arrow) = raw.find(" => ") {
        return raw[arrow + " => ".len()..].trim().to_string();
    }
    raw.to_string()
}

/// Parse the `--stat` summary line's `N insertions(+), M deletions(-)`
/// totals (each defaults to `0` when its sub-field is absent).
fn parse_stat_summary_line(line: &str) -> (u32, u32) {
    let mut insertions = 0u32;
    let mut deletions = 0u32;
    for part in line.split(',') {
        let part = part.trim();
        if part.contains("insertion") {
            insertions = part
                .split_whitespace()
                .next()
                .and_then(|tok| tok.parse().ok())
                .unwrap_or(0);
        } else if part.contains("deletion") {
            deletions = part
                .split_whitespace()
                .next()
                .and_then(|tok| tok.parse().ok())
                .unwrap_or(0);
        }
    }
    (insertions, deletions)
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

    #[test]
    fn numstat_simple_changes() {
        let input = "3\t1\tfoo.txt\x000\t5\tbar.txt\0";
        let diff = parse_diff_numstat(input).unwrap();
        assert_eq!(diff.files.len(), 2);
        assert_eq!(diff.files[0].path, "foo.txt");
        assert_eq!(diff.files[0].insertions, 3);
        assert_eq!(diff.files[0].deletions, 1);
        assert!(!diff.files[0].binary);
        assert_eq!(diff.total_insertions, 3);
        assert_eq!(diff.total_deletions, 6);
    }

    #[test]
    fn numstat_binary_entry() {
        let input = "-\t-\timg.png\0";
        let diff = parse_diff_numstat(input).unwrap();
        assert_eq!(diff.files.len(), 1);
        assert!(diff.files[0].binary);
        assert_eq!(diff.files[0].insertions, 0);
        assert_eq!(diff.files[0].deletions, 0);
        assert_eq!(diff.total_insertions, 0);
    }

    #[test]
    fn numstat_rename_keeps_new_path() {
        let input = "2\t1\t\0old.rs\0new.rs\0";
        let diff = parse_diff_numstat(input).unwrap();
        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.files[0].path, "new.rs");
        assert_eq!(diff.files[0].insertions, 2);
        assert_eq!(diff.files[0].deletions, 1);
    }

    #[test]
    fn numstat_empty_ok() {
        let diff = parse_diff_numstat("").unwrap();
        assert!(diff.files.is_empty());
        assert_eq!(diff.total_insertions, 0);
        assert_eq!(diff.total_deletions, 0);
    }

    #[test]
    fn numstat_invalid_count_errors() {
        assert!(parse_diff_numstat("x\t1\tfoo.txt\0").is_err());
    }

    #[test]
    fn stat_simple_changes_and_summary() {
        let input = " foo.txt | 3 ++-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n";
        let diff = parse_diff_stat(input).unwrap();
        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.files[0].path, "foo.txt");
        assert_eq!(diff.files[0].insertions, 2);
        assert_eq!(diff.files[0].deletions, 1);
        assert_eq!(diff.total_insertions, 2);
        assert_eq!(diff.total_deletions, 1);
    }

    #[test]
    fn stat_binary_entry() {
        let input =
            " bin.dat | Bin 3 -> 5 bytes\n 1 file changed, 0 insertions(+), 0 deletions(-)\n";
        let diff = parse_diff_stat(input).unwrap();
        assert_eq!(diff.files.len(), 1);
        assert!(diff.files[0].binary);
        assert_eq!(diff.files[0].insertions, 0);
        assert_eq!(diff.files[0].deletions, 0);
    }

    #[test]
    fn stat_simple_rename() {
        let input = " old.rs => new.rs | 3 ++-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n";
        let diff = parse_diff_stat(input).unwrap();
        assert_eq!(diff.files[0].path, "new.rs");
    }

    #[test]
    fn stat_abbreviated_rename() {
        let input = " dir/{sub_b => sub_c}/file.txt | 0\n 1 file changed, 0 insertions(+), 0 deletions(-)\n";
        let diff = parse_diff_stat(input).unwrap();
        assert_eq!(diff.files[0].path, "dir/sub_c/file.txt");
    }

    #[test]
    fn stat_insertions_only_summary() {
        let input = " a.txt | 1 +\n 1 file changed, 1 insertion(+)\n";
        let diff = parse_diff_stat(input).unwrap();
        assert_eq!(diff.total_insertions, 1);
        assert_eq!(diff.total_deletions, 0);
    }

    #[test]
    fn stat_empty_ok() {
        let diff = parse_diff_stat("").unwrap();
        assert!(diff.files.is_empty());
        assert_eq!(diff.total_insertions, 0);
        assert_eq!(diff.total_deletions, 0);
    }
}
