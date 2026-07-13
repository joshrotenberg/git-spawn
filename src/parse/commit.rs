//! Parser for `git commit` output.
//!
//! `git commit` prints a header line — `[branch hash] subject` or, for the
//! first commit in a repository, `[branch (root-commit) hash] subject` —
//! followed by a stats line such as `N files changed, N insertions(+), N
//! deletions(-)`. The stats line omits whichever of insertions/deletions is
//! zero and uses singular wording (`file changed`, `insertion(+)`) when the
//! count is one.

/// Parsed result of a `git commit` invocation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommitResult {
    /// Branch the commit was made on.
    pub branch: String,
    /// Abbreviated commit hash from the header.
    pub short_hash: String,
    /// Commit subject (trailing text of the header line).
    pub subject: String,
    /// Number of files changed.
    pub files_changed: u32,
    /// Number of inserted lines.
    pub insertions: u32,
    /// Number of deleted lines.
    pub deletions: u32,
}

/// Parse the output of `git commit`.
///
/// Permissive by design: an empty or unparseable header yields empty
/// strings, and a missing stats line (or missing insertions/deletions
/// sub-fields) yields zeros.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_commit;
///
/// let input = "[main abc1234] Fix bug\n 2 files changed, 3 insertions(+), 1 deletion(-)\n";
/// let result = parse_commit(input);
/// assert_eq!(result.branch, "main");
/// assert_eq!(result.short_hash, "abc1234");
/// assert_eq!(result.subject, "Fix bug");
/// assert_eq!(result.files_changed, 2);
/// assert_eq!(result.insertions, 3);
/// assert_eq!(result.deletions, 1);
/// ```
#[must_use]
pub fn parse_commit(stdout: &str) -> CommitResult {
    let header = stdout.lines().next().unwrap_or("").trim();
    let (branch, short_hash, subject) = parse_header(header);
    let (files_changed, insertions, deletions) = stdout
        .lines()
        .find(|line| line.contains("changed"))
        .map(parse_stats_line)
        .unwrap_or((0, 0, 0));

    CommitResult {
        branch,
        short_hash,
        subject,
        files_changed,
        insertions,
        deletions,
    }
}

/// Parse the `[branch hash] subject` / `[branch (root-commit) hash] subject`
/// header line. Returns all-empty fields if `header` doesn't start with `[`
/// or has no closing `]`.
fn parse_header(header: &str) -> (String, String, String) {
    if !header.starts_with('[') {
        return (String::new(), String::new(), String::new());
    }
    let Some(close) = header.find(']') else {
        return (String::new(), String::new(), String::new());
    };

    let bracket_content = &header[1..close];
    let subject = header[close + 1..].trim_start().to_string();

    const ROOT_COMMIT: &str = "(root-commit)";
    let (branch, short_hash) = if let Some(idx) = bracket_content.find(ROOT_COMMIT) {
        let before = bracket_content[..idx].trim();
        let after = bracket_content[idx + ROOT_COMMIT.len()..].trim();
        (before.to_string(), after.to_string())
    } else if let Some(pos) = bracket_content.rfind(' ') {
        (
            bracket_content[..pos].trim().to_string(),
            bracket_content[pos + 1..].trim().to_string(),
        )
    } else {
        (String::new(), bracket_content.trim().to_string())
    };

    (branch, short_hash, subject)
}

/// Parse a `N files changed, N insertions(+), N deletions(-)` stats line.
/// Each count defaults to `0` when its sub-field is absent.
fn parse_stats_line(line: &str) -> (u32, u32, u32) {
    let mut files_changed = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    for part in line.split(',') {
        let part = part.trim();
        if part.contains("changed") {
            files_changed = leading_number(part);
        } else if part.contains("insertion") {
            insertions = leading_number(part);
        } else if part.contains("deletion") {
            deletions = leading_number(part);
        }
    }

    (files_changed, insertions, deletions)
}

/// Parse the leading whitespace-delimited number in `s`, or `0` if there
/// isn't one.
fn leading_number(s: &str) -> u32 {
    s.split_whitespace()
        .next()
        .and_then(|tok| tok.parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_normal_header() {
        let input = "[main abc1234] Fix bug\n 1 file changed, 1 insertion(+)\n";
        let result = parse_commit(input);
        assert_eq!(result.branch, "main");
        assert_eq!(result.short_hash, "abc1234");
        assert_eq!(result.subject, "Fix bug");
        assert_eq!(result.files_changed, 1);
        assert_eq!(result.insertions, 1);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn parses_root_commit_header() {
        let input =
            "[main (root-commit) abc1234] Initial commit\n 2 files changed, 4 insertions(+)\n";
        let result = parse_commit(input);
        assert_eq!(result.branch, "main");
        assert_eq!(result.short_hash, "abc1234");
        assert_eq!(result.subject, "Initial commit");
        assert_eq!(result.files_changed, 2);
        assert_eq!(result.insertions, 4);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn parses_insertions_only() {
        let input = "[feat def5678] Add feature\n 1 file changed, 3 insertions(+)\n";
        let result = parse_commit(input);
        assert_eq!(result.insertions, 3);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn parses_deletions_only() {
        let input = "[feat def5678] Remove feature\n 1 file changed, 5 deletions(-)\n";
        let result = parse_commit(input);
        assert_eq!(result.insertions, 0);
        assert_eq!(result.deletions, 5);
    }

    #[test]
    fn parses_insertions_and_deletions() {
        let input =
            "[feat def5678] Update feature\n 3 files changed, 10 insertions(+), 2 deletions(-)\n";
        let result = parse_commit(input);
        assert_eq!(result.files_changed, 3);
        assert_eq!(result.insertions, 10);
        assert_eq!(result.deletions, 2);
    }

    #[test]
    fn missing_stats_line_yields_zeros() {
        let input = "[main abc1234] Empty commit\n";
        let result = parse_commit(input);
        assert_eq!(result.branch, "main");
        assert_eq!(result.short_hash, "abc1234");
        assert_eq!(result.subject, "Empty commit");
        assert_eq!(result.files_changed, 0);
        assert_eq!(result.insertions, 0);
        assert_eq!(result.deletions, 0);
    }

    #[test]
    fn empty_input_yields_defaults() {
        let result = parse_commit("");
        assert_eq!(result, CommitResult::default());
    }

    #[test]
    fn unparseable_header_yields_empty_strings() {
        let input = "not a commit header\n 1 file changed, 1 insertion(+)\n";
        let result = parse_commit(input);
        assert_eq!(result.branch, "");
        assert_eq!(result.short_hash, "");
        assert_eq!(result.subject, "");
        assert_eq!(result.files_changed, 1);
        assert_eq!(result.insertions, 1);
    }
}
