//! Parser for `git submodule status`.
//!
//! Each line is `<status-char><sha> <path>[ (<describe>)]`, where the
//! leading status char is one of `' '` (current), `'+'` (modified), `'-'`
//! (uninitialized), or `'U'` (merge conflict), immediately followed by the
//! recorded commit SHA with no separator. The trailing `(<describe>)` is
//! present only when `git describe` succeeds against that commit; `--cached`
//! and `--recursive` can shift or omit it entirely. Parsing is permissive:
//! empty input yields an empty list, and a line that does not match the
//! expected shape is kept with the trimmed line as its `path`, an empty
//! `sha`, and `SubmoduleStatus::Current`, rather than being dropped.

/// A single parsed entry from `git submodule status`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SubmoduleEntry {
    /// The recorded commit SHA (leading status char stripped).
    pub sha: String,
    /// The submodule path.
    pub path: String,
    /// The `git describe` output, if present.
    pub describe: Option<String>,
    /// Status of the submodule, from the leading status char.
    pub status: SubmoduleStatus,
}

/// Classification of a `git submodule status` leading status character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SubmoduleStatus {
    /// Checked out commit matches the recorded SHA (`' '`).
    Current,
    /// Checked out commit differs from the recorded SHA (`'+'`).
    Modified,
    /// Submodule is not initialized (`'-'`).
    Uninitialized,
    /// Merge conflict in the submodule (`'U'`).
    Conflict,
}

impl From<char> for SubmoduleStatus {
    fn from(c: char) -> Self {
        match c {
            '+' => Self::Modified,
            '-' => Self::Uninitialized,
            'U' => Self::Conflict,
            _ => Self::Current,
        }
    }
}

/// Parse the output of `git submodule status` (with or without `--cached` /
/// `--recursive`).
///
/// # Example
/// ```
/// use git_spawn::parse::{parse_submodule_status, SubmoduleStatus};
/// let input = " abc123 vendor/lib (v1.2.3)\n+def456 vendor/other\n";
/// let entries = parse_submodule_status(input);
/// assert_eq!(entries.len(), 2);
/// assert_eq!(entries[0].status, SubmoduleStatus::Current);
/// assert_eq!(entries[0].describe.as_deref(), Some("v1.2.3"));
/// assert_eq!(entries[1].status, SubmoduleStatus::Modified);
/// assert_eq!(entries[1].describe, None);
/// ```
#[must_use]
pub fn parse_submodule_status(input: &str) -> Vec<SubmoduleEntry> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_line)
        .collect()
}

fn parse_line(line: &str) -> SubmoduleEntry {
    let mut chars = line.chars();
    let Some(status_char) = chars.next() else {
        return fallback(line);
    };
    let status = SubmoduleStatus::from(status_char);
    let rest = &line[status_char.len_utf8()..];

    let mut parts = rest.splitn(2, char::is_whitespace);
    let (Some(sha), Some(remainder)) = (parts.next(), parts.next()) else {
        return fallback(line);
    };
    if sha.is_empty() {
        return fallback(line);
    }

    let (path, describe) = split_path_describe(remainder.trim_start());
    SubmoduleEntry {
        sha: sha.to_string(),
        path,
        describe,
        status,
    }
}

fn fallback(line: &str) -> SubmoduleEntry {
    SubmoduleEntry {
        sha: String::new(),
        path: line.trim().to_string(),
        describe: None,
        status: SubmoduleStatus::Current,
    }
}

/// Split `"path (describe)"` into `(path, Some(describe))`, or `(path, None)`
/// when there is no trailing parenthesized group.
fn split_path_describe(s: &str) -> (String, Option<String>) {
    if s.ends_with(')') {
        if let Some(idx) = s.rfind(" (") {
            let path = s[..idx].to_string();
            let describe = &s[idx + 2..s.len() - 1];
            let describe = if describe.is_empty() {
                None
            } else {
                Some(describe.to_string())
            };
            return (path, describe);
        }
    }
    (s.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_entry_with_describe() {
        let input = " abc123 vendor/lib (heads/main)\n";
        let entries = parse_submodule_status(input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sha, "abc123");
        assert_eq!(entries[0].path, "vendor/lib");
        assert_eq!(entries[0].describe.as_deref(), Some("heads/main"));
        assert_eq!(entries[0].status, SubmoduleStatus::Current);
    }

    #[test]
    fn parses_modified_status() {
        let entries = parse_submodule_status("+def456 vendor/other\n");
        assert_eq!(entries[0].status, SubmoduleStatus::Modified);
        assert_eq!(entries[0].sha, "def456");
        assert_eq!(entries[0].path, "vendor/other");
        assert_eq!(entries[0].describe, None);
    }

    #[test]
    fn parses_uninitialized_status() {
        let entries =
            parse_submodule_status("-0000000000000000000000000000000000000000 vendor/nope\n");
        assert_eq!(entries[0].status, SubmoduleStatus::Uninitialized);
    }

    #[test]
    fn parses_conflict_status() {
        let entries = parse_submodule_status("Uabc123 vendor/conflicted\n");
        assert_eq!(entries[0].status, SubmoduleStatus::Conflict);
        assert_eq!(entries[0].sha, "abc123");
        assert_eq!(entries[0].path, "vendor/conflicted");
    }

    #[test]
    fn describe_absent_is_none() {
        let entries = parse_submodule_status(" abc123 vendor/lib\n");
        assert_eq!(entries[0].path, "vendor/lib");
        assert_eq!(entries[0].describe, None);
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_submodule_status("").is_empty());
    }

    #[test]
    fn multiple_entries() {
        let input = " abc123 a (v1)\n+def456 b\n-0000000000000000000000000000000000000000 c\n";
        let entries = parse_submodule_status(input);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].path, "b");
        assert_eq!(entries[2].status, SubmoduleStatus::Uninitialized);
    }

    #[test]
    fn malformed_line_falls_back_to_whole_line_as_path() {
        // No SHA/path separation at all.
        let entries = parse_submodule_status("garbage-with-no-fields\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sha, "");
        assert_eq!(entries[0].path, "garbage-with-no-fields");
        assert_eq!(entries[0].status, SubmoduleStatus::Current);
    }

    #[test]
    fn malformed_line_missing_path_falls_back() {
        // Status char and sha, but no path field.
        let entries = parse_submodule_status(" abc123\n");
        assert_eq!(entries[0].sha, "");
        assert_eq!(entries[0].path, "abc123");
        assert_eq!(entries[0].status, SubmoduleStatus::Current);
    }
}
