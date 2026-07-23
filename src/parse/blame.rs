//! Parser for `git blame --porcelain` / `--line-porcelain`.
//!
//! The porcelain format emits one header per source line, followed by the
//! line's content prefixed with a tab. The header is
//! `<sha> <original-line> <final-line>[ <lines-in-group>]`, where the group
//! count appears only on the line that starts a run of consecutive lines from
//! the same commit. Commit metadata (`author`, `summary`, and friends) is
//! written out the first time a commit appears and omitted afterwards, so this
//! parser remembers what it has seen per commit and fills the gaps back in.
//! `--line-porcelain` repeats the metadata on every line, which parses the same
//! way.
//!
//! Only authorship fields are surfaced: `blame` answers "who wrote this line",
//! and the committer block is dropped rather than doubling the entry width.
//! Lines that are neither a header, a known metadata key, nor content are
//! ignored, and empty input yields an empty list.

use std::collections::HashMap;

/// One parsed line from `git blame` porcelain output.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlameEntry {
    /// SHA of the commit the line is attributed to.
    pub sha: String,
    /// The line's number in the original file, as of `sha`.
    pub original_line: u32,
    /// The line's number in the file being blamed.
    pub final_line: u32,
    /// Number of consecutive lines attributed to `sha` starting here.
    /// Present only on the line that starts a group.
    pub line_count: Option<u32>,
    /// Author name.
    pub author: Option<String>,
    /// Author email, with git's surrounding angle brackets removed.
    pub author_mail: Option<String>,
    /// Author time as a Unix timestamp.
    pub author_time: Option<i64>,
    /// The commit's subject line.
    pub summary: Option<String>,
    /// Path the line lived at in `sha`, which differs from the blamed path
    /// when the content was moved or copied.
    pub filename: Option<String>,
    /// Whether `sha` is a boundary commit, meaning blame stopped there rather
    /// than reaching the line's true origin.
    pub boundary: bool,
    /// The line's content, without its leading tab.
    pub content: Option<String>,
}

/// Commit-level fields, remembered so later groups from the same commit can
/// inherit what the porcelain format only writes once.
#[derive(Debug, Clone, Default)]
struct CommitMeta {
    author: Option<String>,
    author_mail: Option<String>,
    author_time: Option<i64>,
    summary: Option<String>,
    filename: Option<String>,
    boundary: bool,
}

impl CommitMeta {
    fn apply(&mut self, line: &str) {
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "author" => self.author = Some(value.to_string()),
            "author-mail" => self.author_mail = Some(strip_angle_brackets(value).to_string()),
            "author-time" => {
                if let Ok(time) = value.parse() {
                    self.author_time = Some(time);
                }
            }
            "summary" => self.summary = Some(value.to_string()),
            "filename" => self.filename = Some(value.to_string()),
            "boundary" => self.boundary = true,
            _ => {}
        }
    }
}

fn strip_angle_brackets(value: &str) -> &str {
    value
        .strip_prefix('<')
        .and_then(|v| v.strip_suffix('>'))
        .unwrap_or(value)
}

/// A header line and the metadata gathered for it so far.
struct Pending {
    sha: String,
    original_line: u32,
    final_line: u32,
    line_count: Option<u32>,
    meta: CommitMeta,
}

/// Parse the output of `git blame --porcelain` or `--line-porcelain`.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_blame;
/// let input = "\
/// 1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 1 1 2
/// author A U Thor
/// author-mail <author@example.com>
/// author-time 1700000000
/// summary add the thing
/// filename hello.txt
/// \thello
/// 1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 2 2
/// \tworld
/// ";
/// let entries = parse_blame(input);
/// assert_eq!(entries.len(), 2);
/// assert_eq!(entries[0].line_count, Some(2));
/// assert_eq!(entries[0].author.as_deref(), Some("A U Thor"));
/// assert_eq!(entries[0].content.as_deref(), Some("hello"));
/// // The second line repeats neither the author nor the group count.
/// assert_eq!(entries[1].line_count, None);
/// assert_eq!(entries[1].author.as_deref(), Some("A U Thor"));
/// assert_eq!(entries[1].final_line, 2);
/// ```
#[must_use]
pub fn parse_blame(input: &str) -> Vec<BlameEntry> {
    let mut entries = Vec::new();
    let mut known: HashMap<String, CommitMeta> = HashMap::new();
    let mut pending: Option<Pending> = None;

    for line in input.lines() {
        if let Some(content) = line.strip_prefix('\t') {
            if let Some(p) = pending.take() {
                entries.push(finish(p, Some(content.to_string()), &mut known));
            }
            continue;
        }

        if let Some((sha, original_line, final_line, line_count)) = parse_header(line) {
            // A header while one is still open means the previous line had no
            // content of its own; keep it rather than dropping the attribution.
            if let Some(p) = pending.take() {
                entries.push(finish(p, None, &mut known));
            }
            let meta = known.get(&sha).cloned().unwrap_or_default();
            pending = Some(Pending {
                sha,
                original_line,
                final_line,
                line_count,
                meta,
            });
            continue;
        }

        if let Some(p) = pending.as_mut() {
            p.meta.apply(line);
        }
    }

    if let Some(p) = pending.take() {
        entries.push(finish(p, None, &mut known));
    }

    entries
}

fn finish(
    pending: Pending,
    content: Option<String>,
    known: &mut HashMap<String, CommitMeta>,
) -> BlameEntry {
    let Pending {
        sha,
        original_line,
        final_line,
        line_count,
        meta,
    } = pending;
    known.insert(sha.clone(), meta.clone());
    BlameEntry {
        sha,
        original_line,
        final_line,
        line_count,
        author: meta.author,
        author_mail: meta.author_mail,
        author_time: meta.author_time,
        summary: meta.summary,
        filename: meta.filename,
        boundary: meta.boundary,
        content,
    }
}

fn parse_header(line: &str) -> Option<(String, u32, u32, Option<u32>)> {
    let mut parts = line.split(' ');
    let sha = parts.next()?;
    if sha.len() < 8 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let original_line = parts.next()?.parse().ok()?;
    let final_line = parts.next()?.parse().ok()?;
    let line_count = match parts.next() {
        Some(count) => Some(count.parse().ok()?),
        None => None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((sha.to_string(), original_line, final_line, line_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b";
    const OTHER: &str = "0b9a8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a";

    fn group(sha: &str, original: u32, final_line: u32, count: u32, name: &str) -> String {
        format!(
            "{sha} {original} {final_line} {count}\n\
             author {name}\n\
             author-mail <{name}@example.com>\n\
             author-time 1700000000\n\
             author-tz +0000\n\
             committer {name}\n\
             summary a change\n\
             filename hello.txt\n"
        )
    }

    #[test]
    fn parses_a_single_group() {
        let input = format!("{}\tthe line\n", group(SHA, 1, 1, 1, "Ann"));
        let entries = parse_blame(&input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sha, SHA);
        assert_eq!(entries[0].original_line, 1);
        assert_eq!(entries[0].final_line, 1);
        assert_eq!(entries[0].line_count, Some(1));
        assert_eq!(entries[0].author.as_deref(), Some("Ann"));
        assert_eq!(entries[0].author_mail.as_deref(), Some("Ann@example.com"));
        assert_eq!(entries[0].author_time, Some(1_700_000_000));
        assert_eq!(entries[0].summary.as_deref(), Some("a change"));
        assert_eq!(entries[0].filename.as_deref(), Some("hello.txt"));
        assert_eq!(entries[0].content.as_deref(), Some("the line"));
        assert!(!entries[0].boundary);
    }

    #[test]
    fn continuation_lines_inherit_the_commit_metadata() {
        let input = format!(
            "{}\tfirst\n{SHA} 2 2\n\tsecond\n",
            group(SHA, 1, 1, 2, "Ann")
        );
        let entries = parse_blame(&input);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].line_count, None);
        assert_eq!(entries[1].author.as_deref(), Some("Ann"));
        assert_eq!(entries[1].filename.as_deref(), Some("hello.txt"));
        assert_eq!(entries[1].content.as_deref(), Some("second"));
    }

    #[test]
    fn a_repeated_commit_inherits_metadata_it_no_longer_repeats() {
        let input = format!(
            "{}\tfirst\n{}\tsecond\n{SHA} 2 3 1\nfilename hello.txt\n\tthird\n",
            group(SHA, 1, 1, 1, "Ann"),
            group(OTHER, 1, 2, 1, "Bob"),
        );
        let entries = parse_blame(&input);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[1].author.as_deref(), Some("Bob"));
        assert_eq!(entries[2].sha, SHA);
        assert_eq!(entries[2].author.as_deref(), Some("Ann"));
        assert_eq!(entries[2].summary.as_deref(), Some("a change"));
        assert_eq!(entries[2].content.as_deref(), Some("third"));
    }

    #[test]
    fn boundary_commits_are_flagged() {
        let input = format!("{SHA} 1 1 1\nauthor Ann\nboundary\nfilename hello.txt\n\tthe line\n");
        let entries = parse_blame(&input);
        assert!(entries[0].boundary);
    }

    #[test]
    fn tabs_in_content_are_preserved() {
        let input = format!("{}\tif x:\tdo()\n", group(SHA, 1, 1, 1, "Ann"));
        let entries = parse_blame(&input);
        assert_eq!(entries[0].content.as_deref(), Some("if x:\tdo()"));
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_blame("").is_empty());
    }

    #[test]
    fn non_header_leading_lines_are_ignored() {
        let input = format!("not a header\n{}\tthe line\n", group(SHA, 1, 1, 1, "Ann"));
        let entries = parse_blame(&input);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content.as_deref(), Some("the line"));
    }
}
