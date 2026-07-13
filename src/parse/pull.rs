//! Parser for `git pull` output.
//!
//! `git pull` has no stable machine-readable output format, so this parser
//! classifies the combined stdout/stderr text via case-sensitive substring
//! matching, mirroring the `Git.PullResult` struct from the `git_wrapper_ex`
//! Elixir project. The flags are non-exclusive — a merge commit and
//! conflicts, for instance, can both be set on the same pull. This approach
//! is fragile to git localization and to wording changes across git
//! versions; the `raw` field is retained so callers can fall back to their
//! own inspection when the flags don't fit.

/// Classification of `git pull` output.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PullResult {
    /// Output contains `Already up to date`.
    pub already_up_to_date: bool,
    /// Output contains `Fast-forward`.
    pub fast_forward: bool,
    /// Output contains `Merge made by`.
    pub merge_commit: bool,
    /// Output contains `CONFLICT`.
    pub conflicts: bool,
    /// The original text this result was classified from.
    pub raw: String,
}

/// Classify the combined stdout/stderr of a `git pull` invocation.
///
/// Matching is case-sensitive substring search, so it is sensitive to git's
/// locale and to output-wording changes across versions. The flags are
/// non-exclusive.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_pull;
/// let result = parse_pull("Already up to date.\n");
/// assert!(result.already_up_to_date);
/// assert!(!result.fast_forward);
/// ```
#[must_use]
pub fn parse_pull(output: &str) -> PullResult {
    PullResult {
        already_up_to_date: output.contains("Already up to date"),
        fast_forward: output.contains("Fast-forward"),
        merge_commit: output.contains("Merge made by"),
        conflicts: output.contains("CONFLICT"),
        raw: output.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_up_to_date() {
        let result = parse_pull("Already up to date.\n");
        assert!(result.already_up_to_date);
        assert!(!result.fast_forward);
        assert!(!result.merge_commit);
        assert!(!result.conflicts);
    }

    #[test]
    fn fast_forward() {
        let output = "Updating abc123..def456\nFast-forward\n a.txt | 2 +-\n";
        let result = parse_pull(output);
        assert!(result.fast_forward);
        assert!(!result.already_up_to_date);
        assert!(!result.merge_commit);
        assert!(!result.conflicts);
    }

    #[test]
    fn merge_commit() {
        let output = "Merge made by the 'ort' strategy.\n a.txt | 2 +-\n";
        let result = parse_pull(output);
        assert!(result.merge_commit);
        assert!(!result.already_up_to_date);
        assert!(!result.fast_forward);
        assert!(!result.conflicts);
    }

    #[test]
    fn conflicts() {
        let output = "Auto-merging a.txt\nCONFLICT (content): Merge conflict in a.txt\n";
        let result = parse_pull(output);
        assert!(result.conflicts);
        assert!(!result.already_up_to_date);
        assert!(!result.fast_forward);
        assert!(!result.merge_commit);
    }

    #[test]
    fn combination_merge_and_conflicts() {
        let output =
            "Merge made by the 'ort' strategy.\nCONFLICT (content): Merge conflict in a.txt\n";
        let result = parse_pull(output);
        assert!(result.merge_commit);
        assert!(result.conflicts);
        assert!(!result.already_up_to_date);
        assert!(!result.fast_forward);
    }

    #[test]
    fn no_match_retains_raw() {
        let output = "some unrelated output";
        let result = parse_pull(output);
        assert!(!result.already_up_to_date);
        assert!(!result.fast_forward);
        assert!(!result.merge_commit);
        assert!(!result.conflicts);
        assert_eq!(result.raw, output);
    }
}
