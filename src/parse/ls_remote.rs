//! Parser for `git ls-remote`.
//!
//! Each line is `<sha>\t<ref>`. Annotated tags produce a second line for the
//! same tag whose ref name carries a `^{}` suffix and whose SHA is the tagged
//! object rather than the tag object; that suffix is stripped into
//! [`LsRemoteEntry::peeled`] so both lines share one ref name.
//!
//! `--symref` prepends lines of the form `ref: <target>\t<name>` for the
//! remote's symbolic refs. Those carry no SHA, so they are not entries and are
//! dropped; [`parse_ls_remote_symrefs`] reads them instead. Lines without a tab
//! are dropped as well, which covers the trailing blank line and leaves an
//! empty input yielding an empty list.

/// A single ref reported by `git ls-remote`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LsRemoteEntry {
    /// The object the ref points at.
    pub sha: String,
    /// The full ref name, without any `^{}` suffix.
    pub name: String,
    /// Whether this line was the peeled (`^{}`) form of an annotated tag, in
    /// which case `sha` is the tagged commit rather than the tag object.
    pub peeled: bool,
}

/// Parse the output of `git ls-remote`.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_ls_remote;
/// let input = "abc123\trefs/heads/main\ndef456\trefs/tags/v1.0\n0f0f0f\trefs/tags/v1.0^{}\n";
/// let entries = parse_ls_remote(input);
/// assert_eq!(entries.len(), 3);
/// assert_eq!(entries[0].name, "refs/heads/main");
/// assert!(!entries[0].peeled);
/// assert_eq!(entries[2].name, "refs/tags/v1.0");
/// assert_eq!(entries[2].sha, "0f0f0f");
/// assert!(entries[2].peeled);
/// ```
#[must_use]
pub fn parse_ls_remote(input: &str) -> Vec<LsRemoteEntry> {
    input.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<LsRemoteEntry> {
    let (sha, name) = line.split_once('\t')?;
    if sha.is_empty() || name.is_empty() || sha.starts_with("ref: ") {
        return None;
    }
    let (name, peeled) = match name.strip_suffix("^{}") {
        Some(stripped) => (stripped, true),
        None => (name, false),
    };
    Some(LsRemoteEntry {
        sha: sha.to_string(),
        name: name.to_string(),
        peeled,
    })
}

/// Parse the `--symref` lines of `git ls-remote` into `(name, target)` pairs.
///
/// Lines that are ordinary ref entries are skipped, so this can be called on
/// the same output as [`parse_ls_remote`].
///
/// # Example
/// ```
/// use git_spawn::parse::parse_ls_remote_symrefs;
/// let input = "ref: refs/heads/main\tHEAD\nabc123\tHEAD\n";
/// let symrefs = parse_ls_remote_symrefs(input);
/// assert_eq!(symrefs, vec![("HEAD".to_string(), "refs/heads/main".to_string())]);
/// ```
#[must_use]
pub fn parse_ls_remote_symrefs(input: &str) -> Vec<(String, String)> {
    input
        .lines()
        .filter_map(|line| {
            let (target, name) = line.split_once('\t')?;
            let target = target.strip_prefix("ref: ")?;
            if target.is_empty() || name.is_empty() {
                return None;
            }
            Some((name.to_string(), target.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_branch_and_a_tag() {
        let entries = parse_ls_remote("abc123\trefs/heads/main\ndef456\trefs/tags/v1.0\n");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sha, "abc123");
        assert_eq!(entries[0].name, "refs/heads/main");
        assert!(!entries[0].peeled);
        assert_eq!(entries[1].name, "refs/tags/v1.0");
    }

    #[test]
    fn strips_the_peeled_suffix_and_flags_it() {
        let entries = parse_ls_remote("def456\trefs/tags/v1.0\n0f0f0f\trefs/tags/v1.0^{}\n");
        assert_eq!(entries[0].name, entries[1].name);
        assert!(!entries[0].peeled);
        assert!(entries[1].peeled);
        assert_eq!(entries[1].sha, "0f0f0f");
    }

    #[test]
    fn head_is_an_ordinary_entry() {
        let entries = parse_ls_remote("abc123\tHEAD\n");
        assert_eq!(entries[0].name, "HEAD");
    }

    #[test]
    fn symref_lines_are_not_entries() {
        let entries = parse_ls_remote("ref: refs/heads/main\tHEAD\nabc123\tHEAD\n");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sha, "abc123");
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_ls_remote("").is_empty());
        assert!(parse_ls_remote("\n\n").is_empty());
    }

    #[test]
    fn tabless_lines_are_dropped() {
        assert!(parse_ls_remote("From /some/remote\n").is_empty());
    }

    #[test]
    fn symrefs_are_read_back_as_name_target_pairs() {
        let input = "ref: refs/heads/main\tHEAD\nabc123\tHEAD\nabc123\trefs/heads/main\n";
        assert_eq!(
            parse_ls_remote_symrefs(input),
            vec![("HEAD".to_string(), "refs/heads/main".to_string())]
        );
    }

    #[test]
    fn symrefs_are_empty_when_the_flag_was_not_used() {
        assert!(parse_ls_remote_symrefs("abc123\trefs/heads/main\n").is_empty());
    }
}
