//! Parser for `git shortlog` output.
//!
//! Two shapes are covered. The default output writes a header per author,
//! `<ident> (<count>):`, followed by one commit subject per line indented by
//! six spaces; `-w` wraps long subjects onto continuation lines indented by
//! nine, which are rejoined here with a single space. The `-s` output drops
//! the subjects entirely, leaving `<count>\t<ident>` with the count
//! right-aligned.
//!
//! In both shapes `<ident>` is the author name, or `<name> <email>` under
//! `-e`; the email is split out and its angle brackets removed. Lines that
//! match neither shape are ignored, and empty input yields an empty list.

/// Indent git puts before each commit subject in the default output.
const SUBJECT_INDENT: &str = "      ";

/// One author's entry in a `git shortlog` report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShortlogEntry {
    /// Author name, without the email.
    pub author: String,
    /// Author email, present when the report was produced with `-e`.
    pub email: Option<String>,
    /// Number of commits git attributed to the author.
    pub count: u32,
    /// Subjects of those commits, empty when the report used `-s`.
    pub subjects: Vec<String>,
}

/// Parse the output of `git shortlog`.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_shortlog;
/// let input = "\
/// A U Thor <author@example.com> (2):
///       add the thing
///       fix the thing
///
/// ";
/// let entries = parse_shortlog(input);
/// assert_eq!(entries.len(), 1);
/// assert_eq!(entries[0].author, "A U Thor");
/// assert_eq!(entries[0].email.as_deref(), Some("author@example.com"));
/// assert_eq!(entries[0].count, 2);
/// assert_eq!(entries[0].subjects, ["add the thing", "fix the thing"]);
/// ```
#[must_use]
pub fn parse_shortlog(input: &str) -> Vec<ShortlogEntry> {
    let mut entries: Vec<ShortlogEntry> = Vec::new();

    for line in input.lines() {
        if let Some(rest) = line.strip_prefix(SUBJECT_INDENT) {
            if rest.trim().is_empty() {
                continue;
            }
            let Some(entry) = entries.last_mut() else {
                continue;
            };
            // A deeper indent is git's wrap continuation of the subject above.
            match (rest.starts_with(' '), entry.subjects.last_mut()) {
                (true, Some(subject)) => {
                    subject.push(' ');
                    subject.push_str(rest.trim_start());
                }
                _ => entry.subjects.push(rest.to_string()),
            }
            continue;
        }

        if let Some((count, ident)) = parse_summary_line(line) {
            entries.push(new_entry(ident, count));
            continue;
        }

        if let Some((ident, count)) = parse_group_header(line) {
            entries.push(new_entry(ident, count));
        }
    }

    entries
}

fn new_entry(ident: &str, count: u32) -> ShortlogEntry {
    let (author, email) = split_ident(ident);
    ShortlogEntry {
        author,
        email,
        count,
        subjects: Vec::new(),
    }
}

/// `<count>\t<ident>`, as written by `-s`.
fn parse_summary_line(line: &str) -> Option<(u32, &str)> {
    let (count, ident) = line.split_once('\t')?;
    Some((count.trim().parse().ok()?, ident))
}

/// `<ident> (<count>):`, the header of a default-format group.
fn parse_group_header(line: &str) -> Option<(&str, u32)> {
    let head = line.strip_suffix("):")?;
    let (ident, count) = head.rsplit_once(" (")?;
    Some((ident, count.parse().ok()?))
}

/// Split `<name> <email>` into its parts, leaving a bare name's email absent.
fn split_ident(ident: &str) -> (String, Option<String>) {
    let ident = ident.trim();
    if let Some(head) = ident.strip_suffix('>') {
        if let Some((name, email)) = head.rsplit_once(" <") {
            return (name.to_string(), Some(email.to_string()));
        }
    }
    (ident.to_string(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_default_grouped_format() {
        let input = "\
Ann Author (2):
      add one
      add two

Bob Builder (1):
      fix three

";
        let entries = parse_shortlog(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].author, "Ann Author");
        assert_eq!(entries[0].email, None);
        assert_eq!(entries[0].count, 2);
        assert_eq!(entries[0].subjects, ["add one", "add two"]);
        assert_eq!(entries[1].author, "Bob Builder");
        assert_eq!(entries[1].count, 1);
        assert_eq!(entries[1].subjects, ["fix three"]);
    }

    #[test]
    fn parses_the_summary_format() {
        let input = "    12\tAnn Author\n     1\tBob Builder\n";
        let entries = parse_shortlog(input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].author, "Ann Author");
        assert_eq!(entries[0].count, 12);
        assert!(entries[0].subjects.is_empty());
        assert_eq!(entries[1].count, 1);
    }

    #[test]
    fn splits_the_email_out_of_the_identity() {
        let input = "     2\tAnn Author <ann@example.com>\n";
        let entries = parse_shortlog(input);
        assert_eq!(entries[0].author, "Ann Author");
        assert_eq!(entries[0].email.as_deref(), Some("ann@example.com"));

        let grouped = parse_shortlog("Ann Author <ann@example.com> (1):\n      add one\n");
        assert_eq!(grouped[0].author, "Ann Author");
        assert_eq!(grouped[0].email.as_deref(), Some("ann@example.com"));
    }

    #[test]
    fn wrapped_subjects_are_rejoined() {
        let input = "Ann Author (1):\n      a subject long enough that git\n         wrapped it\n";
        let entries = parse_shortlog(input);
        assert_eq!(
            entries[0].subjects,
            ["a subject long enough that git wrapped it"]
        );
    }

    #[test]
    fn a_name_containing_parentheses_keeps_them() {
        let input = "Ann Author (the maintainer) (3):\n      add one\n";
        let entries = parse_shortlog(input);
        assert_eq!(entries[0].author, "Ann Author (the maintainer)");
        assert_eq!(entries[0].count, 3);
    }

    #[test]
    fn subjects_before_any_header_are_ignored() {
        let entries = parse_shortlog("      orphaned subject\nAnn Author (1):\n      add one\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].subjects, ["add one"]);
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_shortlog("").is_empty());
    }
}
