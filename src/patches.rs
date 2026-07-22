//! Typed patch round-tripping: format commits out, replay them back in.
//!
//! Reached through [`Repository::patches`], which returns a [`PatchOps`]
//! handle. [`format`](PatchOps::format) returns a chained builder over
//! `git format-patch`; [`apply`](PatchOps::apply) and [`am`](PatchOps::am)
//! replay a patch into the working tree and into a commit respectively.
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Write the last three commits out as mbox-style patches.
//! let paths = repo
//!     .patches()
//!     .format("HEAD~3..HEAD")
//!     .output_dir("/tmp/p")
//!     .execute()
//!     .await?;
//!
//! // Replay one into the working tree, and one as a commit.
//! repo.patches().apply(&paths[0]).await?;
//! repo.patches().am(&paths[1]).await?;
//! # Ok(())
//! # }
//! ```
//!
//! The difference between the two replay ops is the one git draws: `apply`
//! only changes the working tree (and the index, with
//! [`ApplyCommand::index`]), while `am` reads the patch as a mailbox and
//! records a commit carrying its original author, date, and subject.
//!
//! These helpers cover the common round trip. The raw
//! [`FormatPatchCommand`], [`ApplyCommand`], and [`AmCommand`] builders reach
//! the flags this module does not model, including the `--continue` /
//! `--skip` / `--abort` controls for an `am` session that stopped on a
//! conflict.

use crate::command::GitCommand;
use crate::command::am::AmCommand;
use crate::command::apply::ApplyCommand;
use crate::command::format_patch::FormatPatchCommand;
use crate::error::Result;
use crate::repo::Repository;
use std::path::{Path, PathBuf};

/// High-level patch round-tripping, scoped to a [`Repository`].
///
/// Obtained via [`Repository::patches`].
#[derive(Debug)]
pub struct PatchOps<'a> {
    repo: &'a Repository,
}

impl<'a> PatchOps<'a> {
    /// Format the commits selected by `rev_spec` as patch files.
    ///
    /// Returns a chained builder; call [`execute`](FormatPatches::execute) to
    /// run it.
    #[must_use]
    pub fn format(&self, rev_spec: impl Into<String>) -> FormatPatches<'a> {
        FormatPatches {
            repo: self.repo,
            rev_spec: rev_spec.into(),
            output_dir: None,
            numbered: false,
            signoff: false,
        }
    }

    /// Apply the patch at `path` to the working tree.
    ///
    /// Equivalent to `git apply <path>`. The change is left unstaged and
    /// uncommitted; use [`am`](PatchOps::am) to record it as a commit
    /// instead. Errors if the patch does not apply.
    pub async fn apply(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut cmd = ApplyCommand::new();
        cmd.current_dir(self.repo.path()).patch(path.as_ref());
        cmd.execute().await?;
        Ok(())
    }

    /// Apply the mailbox at `path` and commit each patch it carries.
    ///
    /// Equivalent to `git am <path>`. Each commit keeps the author, date, and
    /// subject recorded in the patch. When a patch does not apply, `git am`
    /// stops and leaves the repository mid-session: this returns the error,
    /// and resolving or unwinding that state needs the `--continue` /
    /// `--skip` / `--abort` controls on [`AmCommand`].
    pub async fn am(&self, path: impl AsRef<Path>) -> Result<()> {
        let mut cmd = AmCommand::new();
        cmd.current_dir(self.repo.path()).mailbox(path.as_ref());
        cmd.execute().await?;
        Ok(())
    }
}

/// Chained builder over `git format-patch`, scoped to a [`Repository`].
///
/// Obtained via [`PatchOps::format`]. Set any options, then call
/// [`execute`](FormatPatches::execute). The handle borrows the repository for
/// the duration of one chained call.
#[derive(Debug)]
pub struct FormatPatches<'a> {
    repo: &'a Repository,
    rev_spec: String,
    output_dir: Option<PathBuf>,
    numbered: bool,
    signoff: bool,
}

impl<'a> FormatPatches<'a> {
    /// Write the generated patches into `dir` instead of the repository root.
    ///
    /// A relative path is resolved against the repository root, matching the
    /// `-o` argument git itself receives.
    #[must_use]
    pub fn output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(dir.into());
        self
    }

    /// Force numbered `[PATCH n/m]` subject prefixes even for a single patch.
    #[must_use]
    pub fn numbered(mut self) -> Self {
        self.numbered = true;
        self
    }

    /// Add a `Signed-off-by` trailer to each generated patch.
    #[must_use]
    pub fn signoff(mut self) -> Self {
        self.signoff = true;
        self
    }

    /// Write the patches and return their paths, in the order git generated
    /// them.
    ///
    /// Git prints the paths relative to the directory it ran in, which is the
    /// repository root; they are returned joined onto that root, so every
    /// path is usable from anywhere in the calling program. An absolute
    /// [`output_dir`](FormatPatches::output_dir) is left as git reported it.
    ///
    /// # Errors
    /// Returns an error if `git format-patch` fails, for example when the
    /// revision range does not resolve.
    pub async fn execute(self) -> Result<Vec<PathBuf>> {
        let root = self.repo.path().to_path_buf();
        let paths = self.command().execute().await?;
        Ok(paths.into_iter().map(|p| root.join(p)).collect())
    }

    /// The configured raw command, split out so tests can pin the argument
    /// vector the helper builds.
    fn command(&self) -> FormatPatchCommand {
        let mut cmd = FormatPatchCommand::new();
        cmd.current_dir(self.repo.path());
        cmd.rev_spec(&self.rev_spec);
        if let Some(dir) = &self.output_dir {
            cmd.output_dir(dir);
        }
        if self.numbered {
            cmd.numbered();
        }
        if self.signoff {
            cmd.signoff();
        }
        cmd
    }
}

impl Repository {
    /// Typed patch round-tripping over `format-patch`, `apply`, and `am`.
    #[must_use]
    pub fn patches(&self) -> PatchOps<'_> {
        PatchOps { repo: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_builds_the_expected_args() {
        let repo = Repository::new_unchecked("/tmp/some-repo");
        let cmd = repo
            .patches()
            .format("HEAD~2..HEAD")
            .output_dir("/tmp/p")
            .numbered()
            .signoff()
            .command();

        assert_eq!(
            cmd.get_executor().cwd,
            Some(PathBuf::from("/tmp/some-repo"))
        );
        assert_eq!(
            cmd.build_command_args(),
            vec![
                "format-patch",
                "-n",
                "--signoff",
                "-o",
                "/tmp/p",
                "HEAD~2..HEAD"
            ]
        );
    }

    #[test]
    fn format_without_options_passes_only_the_rev_spec() {
        let repo = Repository::new_unchecked("/tmp/some-repo");
        let cmd = repo.patches().format("HEAD~1..HEAD").command();
        assert_eq!(
            cmd.build_command_args(),
            vec!["format-patch", "HEAD~1..HEAD"]
        );
    }
}
