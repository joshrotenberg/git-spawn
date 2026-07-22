//! Parser for `git cherry`.
//!
//! Each line is `<+|-> <sha>`, with the commit's subject appended when the
//! command ran with `-v`. The leading marker is `+` when the commit has no
//! equivalent upstream and `-` when an equivalent patch is already there.
//! Unlike most of git's porcelain this format is stable and strict, so lines
//! that do not begin with `+ ` or `- ` are dropped rather than kept as a
//! fallback entry; empty input yields an empty list.

/// A single parsed entry from `git cherry`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CherryEntry {
    /// Whether an equivalent patch already exists upstream.
    pub status: CherryStatus,
    /// The commit SHA.
    pub sha: String,
    /// The subject line, present only when the command ran with `-v`.
    pub subject: Option<String>,
}

/// Classification of a `git cherry` leading marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CherryStatus {
    /// No equivalent patch upstream yet (`'+'`).
    NotUpstream,
    /// An equivalent patch is already upstream (`'-'`).
    Upstream,
}

/// Parse the output of `git cherry` (with or without `-v`).
///
/// # Example
/// ```
/// use git_spawn::parse::{parse_cherry, CherryStatus};
/// let input = "+ abc123 add the thing\n- def456 fix the thing\n";
/// let entries = parse_cherry(input);
/// assert_eq!(entries.len(), 2);
/// assert_eq!(entries[0].status, CherryStatus::NotUpstream);
/// assert_eq!(entries[0].sha, "abc123");
/// assert_eq!(entries[0].subject.as_deref(), Some("add the thing"));
/// assert_eq!(entries[1].status, CherryStatus::Upstream);
/// assert_eq!(entries[1].subject.as_deref(), Some("fix the thing"));
/// ```
#[must_use]
pub fn parse_cherry(input: &str) -> Vec<CherryEntry> {
    input.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<CherryEntry> {
    let status = match line.chars().next()? {
        '+' => CherryStatus::NotUpstream,
        '-' => CherryStatus::Upstream,
        _ => return None,
    };
    let rest = line[1..].strip_prefix(' ')?;

    let mut parts = rest.splitn(2, ' ');
    let sha = parts.next()?;
    if sha.is_empty() {
        return None;
    }
    let subject = parts.next().map(str::trim).filter(|s| !s.is_empty());

    Some(CherryEntry {
        status,
        sha: sha.to_string(),
        subject: subject.map(ToString::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_markers_without_subjects() {
        let entries = parse_cherry("+ abc123\n- def456\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].status, CherryStatus::NotUpstream);
        assert_eq!(entries[0].sha, "abc123");
        assert_eq!(entries[0].subject, None);
        assert_eq!(entries[1].status, CherryStatus::Upstream);
        assert_eq!(entries[1].sha, "def456");
    }

    #[test]
    fn parses_verbose_subject() {
        let entries = parse_cherry("+ abc123 add the thing\n");
        assert_eq!(entries[0].subject.as_deref(), Some("add the thing"));
    }

    #[test]
    fn subject_keeps_interior_spacing() {
        let entries = parse_cherry("+ abc123 fix:  two  spaces\n");
        assert_eq!(entries[0].subject.as_deref(), Some("fix:  two  spaces"));
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_cherry("").is_empty());
    }

    #[test]
    fn unmarked_lines_are_dropped() {
        let entries = parse_cherry("abc123 no marker\n\n+ def456\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sha, "def456");
    }

    #[test]
    fn marker_without_a_sha_is_dropped() {
        assert!(parse_cherry("+\n+ \n").is_empty());
    }
}
