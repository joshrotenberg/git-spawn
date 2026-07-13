//! Parser for `git cherry-pick` output.
//!
//! `git cherry-pick` has no stable machine-readable output format, so this
//! parser classifies the combined stdout/stderr text via a substring check,
//! mirroring the `Git.CherryPickResult` struct from the `git_wrapper_ex`
//! Elixir project. This is fragile to git localization and to wording
//! changes across git versions; the `raw` field is retained so callers can
//! fall back to their own inspection when the flag doesn't fit.

use crate::command::CommandOutput;

/// Outcome of a `git cherry-pick` invocation, parsed from its output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CherryPickResult {
    /// True if the pick stopped on a conflict (heuristic: output contains
    /// `"CONFLICT"`).
    pub conflicts: bool,
    /// Raw combined stdout/stderr text retained for callers that need it.
    pub raw: String,
}

/// Classify the stdout/stderr of a `git cherry-pick` invocation into a
/// [`CherryPickResult`].
///
/// Matching is case-sensitive substring search, so it is sensitive to git's
/// locale and to output-wording changes across versions.
///
/// # Example
/// ```
/// use git_spawn::CommandOutput;
/// use git_spawn::parse::parse_cherry_pick;
///
/// let output = CommandOutput {
///     stdout: b"Auto-merging a.txt\nCONFLICT (content): Merge conflict in a.txt\n".to_vec(),
///     stderr: String::new(),
///     exit_code: 1,
///     success: false,
/// };
/// let result = parse_cherry_pick(&output);
/// assert!(result.conflicts);
/// ```
#[must_use]
pub fn parse_cherry_pick(output: &CommandOutput) -> CherryPickResult {
    let raw = format!("{}{}", output.stdout_str(), output.stderr);
    CherryPickResult {
        conflicts: raw.contains("CONFLICT"),
        raw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(stdout: &str, stderr: &str, success: bool) -> CommandOutput {
        CommandOutput {
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.to_string(),
            exit_code: i32::from(!success),
            success,
        }
    }

    #[test]
    fn clean_pick_has_no_conflicts() {
        let result = parse_cherry_pick(&output(
            "[main abc1234] add feature\n 1 file changed, 1 insertion(+)\n",
            "",
            true,
        ));
        assert!(!result.conflicts);
        assert!(result.raw.contains("add feature"));
    }

    #[test]
    fn conflicted_pick_is_detected() {
        let result = parse_cherry_pick(&output(
            "Auto-merging a.txt\n",
            "error: could not apply abc1234... add feature\nhint: after resolving the conflicts, mark the corrected paths\nCONFLICT (content): Merge conflict in a.txt\n",
            false,
        ));
        assert!(result.conflicts);
        assert!(result.raw.contains("Auto-merging"));
        assert!(result.raw.contains("CONFLICT"));
    }
}
