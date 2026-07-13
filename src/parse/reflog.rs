//! Parser for `git reflog show` output using a fixed token format.
//!
//! Use [`REFLOG_FORMAT`] as `--format=<fmt>` so `reflog show` emits stable,
//! parseable records: fields separated by NUL `\x1f` (unit separator) and
//! entries separated by NUL `\x1e` (record separator).

use crate::error::{Error, Result};

/// Format string to pass as `--format=<fmt>` so entries parse cleanly.
///
/// Fields (in order): full SHA, short SHA, reflog selector (`%gD`, e.g.
/// `HEAD@{0}`), reflog subject (`%gs`). The subject is split on the first
/// `": "` into `action` and `message` during parsing.
pub const REFLOG_FORMAT: &str = "%H\x1f%h\x1f%gD\x1f%gs\x1e";

/// A single parsed reflog entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ReflogEntry {
    /// Full SHA.
    pub hash: String,
    /// Abbreviated SHA.
    pub abbreviated_hash: String,
    /// Reflog selector, e.g. `HEAD@{0}`.
    pub selector: String,
    /// Reflog action, e.g. `commit`, `checkout`, `reset`.
    pub action: String,
    /// Remainder of the reflog subject after the action.
    pub message: String,
}

/// Parse the output of `git reflog show --format=<REFLOG_FORMAT>`.
///
/// # Errors
/// Returns [`Error::ParseError`] if any record has fewer fields than expected.
///
/// # Example
/// ```
/// use git_spawn::parse::{parse_reflog, REFLOG_FORMAT};
/// # let _ = REFLOG_FORMAT; // silence unused if not used at doc-time
/// let input = "abc\x1fabc\x1fHEAD@{0}\x1fcommit: hi\x1e";
/// let entries = parse_reflog(input).unwrap();
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].action, "commit");
/// assert_eq!(entries[0].message, "hi");
/// ```
pub fn parse_reflog(input: &str) -> Result<Vec<ReflogEntry>> {
    let mut out = Vec::new();
    for record in input.split('\x1e') {
        let trimmed = record.trim_matches('\n');
        if trimmed.is_empty() {
            continue;
        }
        let fields: Vec<&str> = trimmed.split('\x1f').collect();
        if fields.len() < 4 {
            return Err(Error::parse_error(format!(
                "expected 4 fields, got {}",
                fields.len()
            )));
        }
        let subject = fields[3..].join("\x1f");
        let (action, message) = match subject.split_once(": ") {
            Some((action, message)) => (action.to_string(), message.to_string()),
            None => (subject, String::new()),
        };
        out.push(ReflogEntry {
            hash: fields[0].to_string(),
            abbreviated_hash: fields[1].to_string(),
            selector: fields[2].to_string(),
            action,
            message,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_entries() {
        let input = concat!(
            "a\x1fa\x1fHEAD@{0}\x1fcommit: second\x1e",
            "b\x1fb\x1fHEAD@{1}\x1fcheckout: moving from main to topic\x1e",
        );
        let out = parse_reflog(input).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].hash, "a");
        assert_eq!(out[0].selector, "HEAD@{0}");
        assert_eq!(out[0].action, "commit");
        assert_eq!(out[0].message, "second");
        assert_eq!(out[1].action, "checkout");
        assert_eq!(out[1].message, "moving from main to topic");
    }

    #[test]
    fn subject_without_colon_becomes_whole_action() {
        let input = "abc\x1fabc\x1fHEAD@{0}\x1fsome subject with no colon\x1e";
        let out = parse_reflog(input).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].action, "some subject with no colon");
        assert_eq!(out[0].message, "");
    }

    #[test]
    fn empty_message_after_colon() {
        let input = "abc\x1fabc\x1fHEAD@{0}\x1frebase (finish): \x1e";
        let out = parse_reflog(input).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].action, "rebase (finish)");
        assert_eq!(out[0].message, "");
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_reflog("").unwrap().is_empty());
    }

    #[test]
    fn too_few_fields_errors() {
        assert!(parse_reflog("a\x1fb\x1e").is_err());
    }
}
