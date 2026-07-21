//! Parser for `git bisect` output.
//!
//! `git bisect` has no stable machine-readable output format, so this parser
//! classifies its output via substring and line matching, mirroring the
//! `Git.BisectResult` struct from the `git_wrapper_ex` Elixir project. A
//! bisect session moves through phases as commits are marked good/bad:
//!
//! - `start`: no distinguishing markers yet (or too few revisions given to
//!   narrow down).
//! - stepping: `Bisecting: N revisions left to test (roughly M steps)`
//!   followed by a `[<sha>] <subject>` line naming the commit git checked
//!   out next.
//! - found: `<sha> is the first bad commit`, reported once the session
//!   converges.
//! - `reset`: git prints `Previous HEAD position was ...` (or `HEAD is now
//!   at ...`) while restoring the original `HEAD`.
//!
//! This is fragile to git localization and to wording changes across git
//! versions; the `raw` field is retained so callers can fall back to their
//! own inspection when the flags don't fit.

/// Phase of a `git bisect` session, classified from command output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BisectStatus {
    /// Session started; not enough good/bad commits given yet to narrow down.
    Started,
    /// Narrowing down: git checked out a new commit to test.
    Stepping,
    /// Session converged on a first-bad commit.
    Found,
    /// Session ended and the original `HEAD` was restored.
    Done,
}

/// Parsed result of a `git bisect` step, classified from stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BisectResult {
    /// Session phase.
    pub status: BisectStatus,
    /// Commit git checked out for the next test step, if this output
    /// reported one (only set when `status` is [`BisectStatus::Stepping`]).
    pub current_commit: Option<String>,
    /// SHA of the first bad commit, once the session converges (only set
    /// when `status` is [`BisectStatus::Found`]).
    pub bad_commit: Option<String>,
    /// The original text this result was classified from.
    pub raw: String,
}

/// Classify the output of a `git bisect` invocation into a [`BisectResult`].
///
/// Pass the command's stdout and stderr together: git writes bisect progress
/// to stderr on recent versions and to stdout on older ones.
/// [`BisectCommand::parse_result`](crate::command::bisect::BisectCommand::parse_result)
/// concatenates both streams before calling this.
///
/// Matching is substring and line based, so it is sensitive to git's locale
/// and to output-wording changes across versions.
///
/// # Example
/// ```
/// use git_spawn::parse::{BisectStatus, parse_bisect};
/// let result = parse_bisect("Bisecting: 1 revision left to test after this (roughly 1 step)\n[abc1234] some commit\n");
/// assert_eq!(result.status, BisectStatus::Stepping);
/// assert_eq!(result.current_commit.as_deref(), Some("abc1234"));
/// ```
#[must_use]
pub fn parse_bisect(output: &str) -> BisectResult {
    let raw = output.to_string();

    if let Some(bad_commit) = first_bad_commit(output) {
        return BisectResult {
            status: BisectStatus::Found,
            current_commit: None,
            bad_commit: Some(bad_commit),
            raw,
        };
    }

    if output.contains("Bisecting:") {
        return BisectResult {
            status: BisectStatus::Stepping,
            current_commit: checked_out_commit(output),
            bad_commit: None,
            raw,
        };
    }

    if output.contains("Previous HEAD position was") || output.contains("HEAD is now at") {
        return BisectResult {
            status: BisectStatus::Done,
            current_commit: None,
            bad_commit: None,
            raw,
        };
    }

    BisectResult {
        status: BisectStatus::Started,
        current_commit: None,
        bad_commit: None,
        raw,
    }
}

/// Extract the SHA from a `<sha> is the first bad commit` line.
fn first_bad_commit(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let sha = line.strip_suffix(" is the first bad commit")?;
        (!sha.is_empty()).then(|| sha.to_string())
    })
}

/// Extract the SHA from a `[<sha>] <subject>` line, as printed after a
/// `Bisecting:` header.
fn checked_out_commit(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix('[')?;
        let (sha, _) = rest.split_once(']')?;
        (!sha.is_empty()).then(|| sha.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn started_when_no_markers_present() {
        let result = parse_bisect("");
        assert_eq!(result.status, BisectStatus::Started);
        assert!(result.current_commit.is_none());
        assert!(result.bad_commit.is_none());
    }

    #[test]
    fn stepping_extracts_current_commit() {
        let output = "Bisecting: 2 revisions left to test after this (roughly 2 steps)\n[3050fc6d1234567890abcdef1234567890abcdef] c3\n";
        let result = parse_bisect(output);
        assert_eq!(result.status, BisectStatus::Stepping);
        assert_eq!(
            result.current_commit.as_deref(),
            Some("3050fc6d1234567890abcdef1234567890abcdef")
        );
        assert!(result.bad_commit.is_none());
    }

    #[test]
    fn found_extracts_bad_commit() {
        let output = "3050fc6d1234567890abcdef1234567890abcdef is the first bad commit\ncommit 3050fc6d1234567890abcdef1234567890abcdef\n";
        let result = parse_bisect(output);
        assert_eq!(result.status, BisectStatus::Found);
        assert_eq!(
            result.bad_commit.as_deref(),
            Some("3050fc6d1234567890abcdef1234567890abcdef")
        );
        assert!(result.current_commit.is_none());
    }

    #[test]
    fn done_on_reset_output() {
        let output = "Previous HEAD position was 3050fc6 c3\nSwitched to branch 'main'\n";
        let result = parse_bisect(output);
        assert_eq!(result.status, BisectStatus::Done);
    }

    #[test]
    fn done_on_head_is_now_at() {
        let output = "HEAD is now at 3050fc6 c3\n";
        let result = parse_bisect(output);
        assert_eq!(result.status, BisectStatus::Done);
    }

    #[test]
    fn raw_is_preserved() {
        let output = "some unrelated output";
        let result = parse_bisect(output);
        assert_eq!(result.status, BisectStatus::Started);
        assert_eq!(result.raw, output);
    }
}
