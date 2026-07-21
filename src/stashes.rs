//! Typed access to the stash stack.
//!
//! Reached through [`Repository::stashes`], which returns a [`StashOps`]
//! handle. [`list`](StashOps::list) parses `git stash list` into typed
//! [`StashEntry`] values; the mutating helpers ([`push`](StashOps::push),
//! [`pop`](StashOps::pop), and friends) delegate to the raw
//! [`StashCommand`].
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Stash the current changes with a message.
//! repo.stashes().push("wip on review").await?;
//!
//! // Inspect the stack.
//! for entry in repo.stashes().list().await? {
//!     println!(
//!         "stash@{{{}}}  {}  {}",
//!         entry.index, entry.branch, entry.subject
//!     );
//! }
//!
//! // Restore the most recent stash.
//! repo.stashes().pop(0).await?;
//! # Ok(())
//! # }
//! ```

use crate::command::GitCommand;
use crate::command::stash::StashCommand;
use crate::error::Result;
use crate::repo::Repository;

/// A single entry on the stash stack, as reported by `git stash list`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StashEntry {
    /// Stack position: the `N` in `stash@{N}`. `0` is the most recent stash.
    pub index: usize,
    /// Branch the stash was created on, e.g. `main`. Git reports `(no branch)`
    /// for a stash made on a detached HEAD.
    pub branch: String,
    /// The stash subject: a custom message when one was given, otherwise the
    /// auto-generated `<short-sha> <commit-subject>`.
    pub subject: String,
    /// Full commit SHA of the stash commit.
    pub sha: String,
}

/// High-level stash inspection and manipulation, scoped to a [`Repository`].
#[derive(Debug)]
pub struct StashOps<'a> {
    repo: &'a Repository,
}

impl<'a> StashOps<'a> {
    /// List the stash stack, most recent first.
    ///
    /// Runs `git stash list` with a machine-readable format and parses each
    /// entry into a [`StashEntry`]. An empty stack yields an empty vector.
    pub async fn list(&self) -> Result<Vec<StashEntry>> {
        // NUL-terminated records, unit-separated fields: reflog selector,
        // full SHA, reflog subject.
        let mut cmd = self.repo.stash(StashCommand::list());
        cmd.args(["-z", "--format=%gd%x1f%H%x1f%gs"]);
        let out = cmd.execute().await?;
        Ok(parse_stash_list(&out.stdout_str()))
    }

    /// Stash the working-tree changes with `message`.
    ///
    /// Equivalent to `git stash push -m <message>`. Errors if there is nothing
    /// to stash.
    pub async fn push(&self, message: impl Into<String>) -> Result<()> {
        let mut cmd = StashCommand::push();
        cmd.message(message);
        self.repo.stash(cmd).execute().await?;
        Ok(())
    }

    /// Apply the stash at `index` and remove it from the stack.
    ///
    /// Equivalent to `git stash pop stash@{<index>}`.
    pub async fn pop(&self, index: usize) -> Result<()> {
        let cmd = StashCommand::pop(Some(stash_ref(index)));
        self.repo.stash(cmd).execute().await?;
        Ok(())
    }

    /// Apply the stash at `index`, leaving it on the stack.
    ///
    /// Equivalent to `git stash apply stash@{<index>}`.
    pub async fn apply(&self, index: usize) -> Result<()> {
        let cmd = StashCommand::apply(Some(stash_ref(index)));
        self.repo.stash(cmd).execute().await?;
        Ok(())
    }

    /// Drop the stash at `index` without applying it.
    ///
    /// Equivalent to `git stash drop stash@{<index>}`.
    pub async fn drop(&self, index: usize) -> Result<()> {
        let cmd = StashCommand::drop_stash(Some(stash_ref(index)));
        self.repo.stash(cmd).execute().await?;
        Ok(())
    }

    /// Remove every entry from the stash stack.
    ///
    /// Equivalent to `git stash clear`.
    pub async fn clear(&self) -> Result<()> {
        self.repo.stash(StashCommand::clear()).execute().await?;
        Ok(())
    }
}

impl Repository {
    /// Typed access to the stash stack.
    #[must_use]
    pub fn stashes(&self) -> StashOps<'_> {
        StashOps { repo: self }
    }
}

/// `stash@{<index>}`, the revision syntax the raw command expects.
fn stash_ref(index: usize) -> String {
    format!("stash@{{{index}}}")
}

/// Parse the NUL-terminated, unit-separated output of
/// `git stash list -z --format=%gd%x1f%H%x1f%gs`.
fn parse_stash_list(stdout: &str) -> Vec<StashEntry> {
    stdout
        .split('\0')
        .filter(|record| !record.is_empty())
        .filter_map(parse_entry)
        .collect()
}

/// Parse one `stash@{N}<US><sha><US><reflog subject>` record.
fn parse_entry(record: &str) -> Option<StashEntry> {
    let mut fields = record.splitn(3, '\u{1f}');
    let selector = fields.next()?;
    let sha = fields.next()?;
    let raw_subject = fields.next()?;

    let index = selector
        .split_once('{')
        .and_then(|(_, rest)| rest.strip_suffix('}'))
        .and_then(|n| n.parse::<usize>().ok())?;

    // Reflog subjects look like `WIP on main: <auto>` or `On main: <message>`.
    let (branch, subject) = match raw_subject.split_once(": ") {
        Some((prefix, rest)) => {
            let branch = prefix
                .strip_prefix("WIP on ")
                .or_else(|| prefix.strip_prefix("On "))
                .unwrap_or(prefix);
            (branch.to_string(), rest.to_string())
        }
        None => (String::new(), raw_subject.to_string()),
    };

    Some(StashEntry {
        index,
        branch,
        subject,
        sha: sha.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stash_ref_formats_revision() {
        assert_eq!(stash_ref(0), "stash@{0}");
        assert_eq!(stash_ref(3), "stash@{3}");
    }

    #[test]
    fn parses_custom_and_auto_entries() {
        let stdout = "stash@{0}\u{1f}aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\u{1f}On main: wip on review\0\
                      stash@{1}\u{1f}bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\u{1f}WIP on feature: 1234567 tweak parser\0";
        let entries = parse_stash_list(stdout);
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].index, 0);
        assert_eq!(entries[0].branch, "main");
        assert_eq!(entries[0].subject, "wip on review");
        assert_eq!(entries[0].sha, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");

        assert_eq!(entries[1].index, 1);
        assert_eq!(entries[1].branch, "feature");
        assert_eq!(entries[1].subject, "1234567 tweak parser");
    }

    #[test]
    fn empty_output_is_empty() {
        assert!(parse_stash_list("").is_empty());
    }

    #[test]
    fn subject_with_colon_space_is_preserved() {
        let stdout = "stash@{0}\u{1f}cccccccccccccccccccccccccccccccccccccccc\u{1f}On main: fix: off-by-one\0";
        let entries = parse_stash_list(stdout);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].branch, "main");
        assert_eq!(entries[0].subject, "fix: off-by-one");
    }

    #[test]
    fn detached_head_branch_is_kept_verbatim() {
        let stdout = "stash@{0}\u{1f}dddddddddddddddddddddddddddddddddddddddd\u{1f}WIP on (no branch): 89abcde tweak\0";
        let entries = parse_stash_list(stdout);
        assert_eq!(entries[0].branch, "(no branch)");
        assert_eq!(entries[0].subject, "89abcde tweak");
    }
}
