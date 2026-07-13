//! Parser for `git show` output.
//!
//! `git show` prints a commit header followed by a diff and, when `--stat`
//! is requested, a stat block instead of (or, with an explicit `--patch`,
//! alongside) the diff. To make the header parseable, callers that want a
//! [`ShowResult`] should run `show` with `--format=<LOG_FORMAT>` (see
//! [`crate::parse::LOG_FORMAT`]) so the header can be decoded with the same
//! logic that backs [`parse_log`]. A caller-supplied
//! custom `--format` (or `--oneline`) can't be parsed generically, so in that
//! case only `raw` is populated.

use super::log::{CommitEntry, parse_log};
use crate::error::Result;

/// Parsed result of a `git show` invocation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShowResult {
    /// Commit header, present for default-format runs.
    pub commit: Option<CommitEntry>,
    /// Diff body, split from the header/stat. Empty when `--stat` was
    /// requested without an explicit `--patch`, since git omits the diff
    /// in that case.
    pub diff: String,
    /// The `--stat` block. `Some` only when `--stat` was requested,
    /// `None` when it wasn't (distinct from an empty stat block).
    pub stat: Option<String>,
    /// Full unparsed stdout. Always populated; the only populated field
    /// when a custom `--format` (or `--oneline`) was set.
    pub raw: String,
}

/// Parse the output of a `git show` invocation.
///
/// `has_stat` should reflect whether `--stat` was passed to `show`, and
/// `custom_format` whether the caller supplied their own `--format` (or
/// `--oneline`) rather than the internal `LOG_FORMAT`-based one. When
/// `custom_format` is `true`, only `raw` is populated: a custom format's
/// header can't be parsed generically.
///
/// # Errors
/// Returns [`Error::ParseError`](crate::error::Error::ParseError) if the
/// header (once split out) doesn't have the fields `parse_log` expects.
///
/// # Example
/// ```
/// use git_spawn::parse::{parse_show, LOG_FORMAT};
/// # let _ = LOG_FORMAT; // silence unused if not used at doc-time
/// let input = "sha1\x1fsh\x1fA\x1fa@x\x1fd1\x1fB\x1fb@y\x1fd2\x1fhi\x1f\x1e\ndiff --git a/f b/f\n";
/// let result = parse_show(input, false, false).unwrap();
/// assert_eq!(result.commit.unwrap().subject, "hi");
/// assert!(result.diff.starts_with("diff --git"));
/// assert!(result.stat.is_none());
/// ```
pub fn parse_show(stdout: &str, has_stat: bool, custom_format: bool) -> Result<ShowResult> {
    let raw = stdout.to_string();

    if custom_format {
        return Ok(ShowResult {
            commit: None,
            diff: String::new(),
            stat: None,
            raw,
        });
    }

    let Some(sep_idx) = stdout.find('\x1e') else {
        return Ok(ShowResult {
            commit: None,
            diff: stdout.trim_start_matches('\n').to_string(),
            stat: None,
            raw,
        });
    };

    let header = &stdout[..=sep_idx];
    let commit = parse_log(header)?.into_iter().next();
    let remainder = stdout[sep_idx + 1..].trim_start_matches('\n').to_string();

    let (diff, stat) = if has_stat {
        (String::new(), Some(remainder))
    } else {
        (remainder, None)
    };

    Ok(ShowResult {
        commit,
        diff,
        stat,
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(subject: &str) -> String {
        format!(
            "sha1\x1fsh\x1fAlice\x1fa@x\x1f2024-01-01T00:00:00Z\x1fBob\x1fb@y\x1f2024-01-02T00:00:00Z\x1f{subject}\x1f\x1e"
        )
    }

    #[test]
    fn default_format_splits_header_and_diff() {
        let input = format!(
            "{}\ndiff --git a/f b/f\nindex 1..2 100644\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-a\n+b\n",
            header("hello")
        );
        let result = parse_show(&input, false, false).unwrap();
        let commit = result.commit.expect("commit header");
        assert_eq!(commit.sha, "sha1");
        assert_eq!(commit.subject, "hello");
        assert!(result.diff.starts_with("diff --git a/f b/f"));
        assert!(result.stat.is_none());
        assert_eq!(result.raw, input);
    }

    #[test]
    fn stat_present_populates_stat_and_empties_diff() {
        let input = format!(
            "{}\n f | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n",
            header("hello")
        );
        let result = parse_show(&input, true, false).unwrap();
        assert!(result.diff.is_empty());
        let stat = result.stat.expect("stat block");
        assert!(stat.contains("1 file changed"));
    }

    #[test]
    fn stat_absent_is_none() {
        let input = format!("{}\ndiff --git a/f b/f\n", header("hello"));
        let result = parse_show(&input, false, false).unwrap();
        assert!(result.stat.is_none());
    }

    #[test]
    fn custom_format_only_populates_raw() {
        let input = "deadbeef Some custom oneline output\n";
        let result = parse_show(input, false, true).unwrap();
        assert!(result.commit.is_none());
        assert!(result.diff.is_empty());
        assert!(result.stat.is_none());
        assert_eq!(result.raw, input);
    }

    #[test]
    fn no_patch_yields_empty_diff() {
        let input = header("hello");
        let result = parse_show(&input, false, false).unwrap();
        assert!(result.diff.is_empty());
        assert!(result.stat.is_none());
        assert_eq!(result.commit.unwrap().subject, "hello");
    }

    #[test]
    fn missing_separator_falls_back_to_raw_diff() {
        let input = "not a header at all\n";
        let result = parse_show(input, false, false).unwrap();
        assert!(result.commit.is_none());
        assert_eq!(result.diff, input);
        assert_eq!(result.raw, input);
    }

    #[test]
    fn too_few_header_fields_errors() {
        let input = "a\x1fb\x1e\ndiff --git a/f b/f\n";
        assert!(parse_show(input, false, false).is_err());
    }
}
