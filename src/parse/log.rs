//! Parser for `git log` output using a fixed token format.
//!
//! Use [`LOG_FORMAT`] as `--format=<fmt>` so `log` emits stable, parseable
//! records: fields separated by NUL `\x1f` (unit separator) and entries
//! separated by NUL `\x1e` (record separator).

use crate::error::{Error, Result};

/// Format string to pass as `--format=<fmt>` so entries parse cleanly.
///
/// Fields (in order): full SHA, short SHA, author name, author email,
/// author date (ISO 8601 strict), committer name, committer email,
/// committer date, subject, body. Body is last because it may contain
/// newlines; records are terminated by `\x1e`.
pub const LOG_FORMAT: &str = "%H\x1f%h\x1f%an\x1f%ae\x1f%aI\x1f%cn\x1f%ce\x1f%cI\x1f%s\x1f%b\x1e";

/// A single parsed commit entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CommitEntry {
    /// Full SHA.
    pub sha: String,
    /// Abbreviated SHA.
    pub short_sha: String,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Author date, RFC 3339 / ISO 8601 strict.
    pub author_date: String,
    /// Committer name.
    pub committer_name: String,
    /// Committer email.
    pub committer_email: String,
    /// Committer date, RFC 3339 / ISO 8601 strict.
    pub committer_date: String,
    /// Commit subject line.
    pub subject: String,
    /// Commit body (may be empty, may span multiple lines).
    pub body: String,
}

/// Parse the output of `git log --format=<LOG_FORMAT>`.
///
/// # Errors
/// Returns [`Error::ParseError`] if any record has fewer fields than expected.
///
/// # Example
/// ```
/// use git_wrapper::parse::{parse_log, LOG_FORMAT};
/// # let _ = LOG_FORMAT; // silence unused if not used at doc-time
/// let input = "abc\x1fabc\x1fA\x1fa@x\x1f2024-01-01T00:00:00Z\x1fB\x1fb@y\x1f2024-01-02T00:00:00Z\x1fhi\x1fbody\x1e";
/// let commits = parse_log(input).unwrap();
/// assert_eq!(commits.len(), 1);
/// assert_eq!(commits[0].subject, "hi");
/// assert_eq!(commits[0].author_name, "A");
/// ```
pub fn parse_log(input: &str) -> Result<Vec<CommitEntry>> {
    let mut out = Vec::new();
    for record in input.split('\x1e') {
        let trimmed = record.trim_matches('\n');
        if trimmed.is_empty() {
            continue;
        }
        let fields: Vec<&str> = trimmed.split('\x1f').collect();
        if fields.len() < 10 {
            return Err(Error::parse_error(format!(
                "expected 10 fields, got {}",
                fields.len()
            )));
        }
        out.push(CommitEntry {
            sha: fields[0].to_string(),
            short_sha: fields[1].to_string(),
            author_name: fields[2].to_string(),
            author_email: fields[3].to_string(),
            author_date: fields[4].to_string(),
            committer_name: fields[5].to_string(),
            committer_email: fields[6].to_string(),
            committer_date: fields[7].to_string(),
            subject: fields[8].to_string(),
            body: fields[9..].join("\x1f"),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_commit() {
        let input = "sha1\x1fsh\x1fAlice\x1fa@x\x1f2024-01-01T00:00:00Z\x1fBob\x1fb@y\x1f2024-01-02T00:00:00Z\x1fhello\x1fbody text\x1e";
        let out = parse_log(input).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].sha, "sha1");
        assert_eq!(out[0].author_name, "Alice");
        assert_eq!(out[0].body, "body text");
    }

    #[test]
    fn parses_multiple_commits() {
        let input = concat!(
            "a\x1fa\x1fA\x1fa@x\x1fd1\x1fB\x1fb@y\x1fd2\x1fone\x1f\x1e",
            "b\x1fb\x1fA\x1fa@x\x1fd3\x1fB\x1fb@y\x1fd4\x1ftwo\x1f\x1e",
        );
        let out = parse_log(input).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].subject, "one");
        assert_eq!(out[1].subject, "two");
    }

    #[test]
    fn empty_yields_no_commits() {
        assert!(parse_log("").unwrap().is_empty());
    }

    #[test]
    fn too_few_fields_errors() {
        assert!(parse_log("a\x1fb\x1e").is_err());
    }
}
