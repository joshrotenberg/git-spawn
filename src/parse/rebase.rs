//! Parser for `git rebase` output.
//!
//! `git rebase` has no stable machine-readable output format, so this parser
//! classifies the combined stdout/stderr text via case-sensitive substring
//! matching, mirroring the `Git.RebaseResult` struct from the `git_wrapper_ex`
//! Elixir project. Detection is locale-dependent: it matches English message
//! strings and will miss non-English git locales.
//!
//! All three booleans are legitimately `false` on a normal, non-fast-forward
//! successful rebase (`Successfully rebased and updated refs/heads/...`),
//! where only `raw` is meaningful.
//!
//! `--abort` / `--continue` / `--skip` / `--quit` output does not describe a
//! rebase outcome — see
//! [`RebaseCommand::parse_result`](crate::command::rebase::RebaseCommand::parse_result),
//! which guards against misclassifying it.

/// Classification of `git rebase` output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RebaseResult {
    /// Output contains `is up to date`.
    pub up_to_date: bool,
    /// Output contains `Fast-forwarded`.
    pub fast_forward: bool,
    /// Output contains `CONFLICT`.
    pub conflicts: bool,
    /// The original text this result was classified from.
    pub raw: String,
}

/// Classify the combined stdout/stderr of a `git rebase` invocation.
///
/// Matching is case-sensitive substring search, so it is sensitive to git's
/// locale and to output-wording changes across versions. Not meaningful for
/// `--abort` / `--continue` / `--skip` / `--quit` output; prefer
/// [`RebaseCommand::parse_result`](crate::command::rebase::RebaseCommand::parse_result),
/// which skips those cases.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_rebase;
/// let result = parse_rebase("Current branch topic is up to date.\n");
/// assert!(result.up_to_date);
/// assert!(!result.fast_forward);
/// ```
#[must_use]
pub fn parse_rebase(output: &str) -> RebaseResult {
    RebaseResult {
        up_to_date: output.contains("is up to date"),
        fast_forward: output.contains("Fast-forwarded"),
        conflicts: output.contains("CONFLICT"),
        raw: output.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_to_date() {
        let result = parse_rebase("Current branch topic is up to date.\n");
        assert!(result.up_to_date);
        assert!(!result.fast_forward);
        assert!(!result.conflicts);
    }

    #[test]
    fn fast_forward() {
        let output =
            "Successfully rebased and updated refs/heads/topic.\nFast-forwarded topic to main.\n";
        let result = parse_rebase(output);
        assert!(result.fast_forward);
        assert!(!result.up_to_date);
        assert!(!result.conflicts);
    }

    #[test]
    fn conflicts() {
        let output = "Auto-merging a.txt\nCONFLICT (content): Merge conflict in a.txt\nerror: could not apply abc123... second\n";
        let result = parse_rebase(output);
        assert!(result.conflicts);
        assert!(!result.up_to_date);
        assert!(!result.fast_forward);
    }

    #[test]
    fn plain_success_no_flags_set() {
        let output = "Successfully rebased and updated refs/heads/topic.\n";
        let result = parse_rebase(output);
        assert!(!result.up_to_date);
        assert!(!result.fast_forward);
        assert!(!result.conflicts);
        assert_eq!(result.raw, output);
    }
}
