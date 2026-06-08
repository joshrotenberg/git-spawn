//! Parser for `git notes list`.
//!
//! `git notes list` emits one `<note-object-sha> <annotated-object-sha>` pair
//! per line (e.g. `5b8d8870... 04190777...`). This parser turns that into
//! `(note_sha, object_sha)` tuples. It is permissive: blank lines are skipped
//! and lines without a second field are ignored rather than erroring.

/// Parse the output of `git notes list`.
///
/// Returns one `(note_sha, object_sha)` tuple per well-formed line. Lines that
/// do not contain two whitespace-separated fields are skipped.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_notes_list;
/// let input = "5b8d8870 04190777\nabc123 def456\n";
/// let pairs = parse_notes_list(input);
/// assert_eq!(pairs.len(), 2);
/// assert_eq!(pairs[0], ("5b8d8870".to_string(), "04190777".to_string()));
/// assert_eq!(pairs[1].1, "def456");
/// ```
#[must_use]
pub fn parse_notes_list(input: &str) -> Vec<(String, String)> {
    input
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let note = parts.next()?;
            let object = parts.next()?;
            Some((note.to_string(), object.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pairs() {
        let input = "note1 obj1\nnote2 obj2\n";
        let pairs = parse_notes_list(input);
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], ("note1".to_string(), "obj1".to_string()));
        assert_eq!(pairs[1], ("note2".to_string(), "obj2".to_string()));
    }

    #[test]
    fn skips_blank_and_malformed_lines() {
        let input = "\nnote1 obj1\nlonely\n";
        let pairs = parse_notes_list(input);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "note1");
    }

    #[test]
    fn empty_input() {
        assert!(parse_notes_list("").is_empty());
    }
}
