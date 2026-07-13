//! Parser for `git merge` stdout.
//!
//! `git merge` prints a human-readable summary rather than a stable
//! machine format. This parser classifies the two outcomes that are
//! detectable from fixed substrings: a fast-forward (`Fast-forward`) and a
//! no-op merge (`Already up to date.`). Neither substring present means a
//! normal merge commit was created.
//!
//! `--abort` / `--continue` output matches neither substring and does not
//! describe a merge outcome — see
//! [`MergeCommand::parse_result`](crate::command::merge::MergeCommand::parse_result),
//! which guards against misclassifying it.

/// Outcome of a `git merge` invocation, parsed from stdout.
///
/// `fast_forward` and `already_up_to_date` are mutually exclusive; when both
/// are `false`, the merge produced a normal merge commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MergeResult {
    /// Merge resolved as a fast-forward (`stdout` contains `"Fast-forward"`).
    pub fast_forward: bool,
    /// Branch was already up to date (`stdout` contains `"Already up to date."`).
    pub already_up_to_date: bool,
}

/// Parse the stdout of a `git merge` invocation into a [`MergeResult`].
///
/// Not meaningful for `--abort` / `--continue` output; prefer
/// [`MergeCommand::parse_result`](crate::command::merge::MergeCommand::parse_result),
/// which skips those cases.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_merge;
/// let result = parse_merge("Updating a1b2c3..d4e5f6\nFast-forward\n f.txt | 1 +\n");
/// assert!(result.fast_forward);
/// assert!(!result.already_up_to_date);
/// ```
#[must_use]
pub fn parse_merge(stdout: &str) -> MergeResult {
    MergeResult {
        fast_forward: stdout.contains("Fast-forward"),
        already_up_to_date: stdout.contains("Already up to date."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_forward() {
        let stdout = "Updating a1b2c3..d4e5f6\nFast-forward\n f.txt | 1 +\n 1 file changed, 1 insertion(+)\n";
        let result = parse_merge(stdout);
        assert!(result.fast_forward);
        assert!(!result.already_up_to_date);
    }

    #[test]
    fn already_up_to_date() {
        let result = parse_merge("Already up to date.\n");
        assert!(!result.fast_forward);
        assert!(result.already_up_to_date);
    }

    #[test]
    fn merge_commit() {
        let stdout =
            "Merge made by the 'ort' strategy.\n f.txt | 1 +\n 1 file changed, 1 insertion(+)\n";
        let result = parse_merge(stdout);
        assert!(!result.fast_forward);
        assert!(!result.already_up_to_date);
    }
}
