//! Common multi-step compositions, one call apiece.
//!
//! Reached through [`Repository::workflow`], which returns a [`WorkflowOps`]
//! handle. Each method bundles two or three raw commands into a single async
//! call — the same things you'd usually script by hand.
//!
//! ```no_run
//! # async fn ex() -> git_wrapper::Result<()> {
//! use git_wrapper::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Start a new feature branch off main.
//! repo.workflow().feature_branch("feature/widget", "main").await?;
//!
//! // Stage everything dirty and commit it.
//! repo.workflow().commit_all("wip: widget").await?;
//!
//! // Rebase the current branch onto its upstream.
//! repo.workflow().sync().await?;
//! # Ok(())
//! # }
//! ```
//!
//! Workflow methods don't add new behavior — they just spare callers from
//! chaining the existing builders by hand. If you need finer control (e.g.
//! `add -u` instead of `add -A`), reach for the raw commands directly.

use crate::command::GitCommand;
use crate::command::add::AddCommand;
use crate::command::checkout::CheckoutCommand;
use crate::command::commit::CommitCommand;
use crate::command::merge::MergeCommand;
use crate::command::pull::PullCommand;
use crate::error::Result;
use crate::repo::Repository;

/// High-level workflow helpers, scoped to a [`Repository`].
#[derive(Debug)]
pub struct WorkflowOps<'a> {
    repo: &'a Repository,
}

impl<'a> WorkflowOps<'a> {
    /// Create a new branch `name` starting at `base` and switch to it.
    ///
    /// Equivalent to `git checkout -b <name> <base>`. Errors if a branch
    /// named `name` already exists.
    pub async fn feature_branch(
        &self,
        name: impl Into<String>,
        base: impl Into<String>,
    ) -> Result<()> {
        let mut cmd = CheckoutCommand::new();
        cmd.create(name).target(base);
        cmd.current_dir(self.repo.path());
        cmd.execute().await?;
        Ok(())
    }

    /// Stage every change in the working tree and commit them with `message`.
    ///
    /// Equivalent to `git add -A` followed by `git commit -m <message>`.
    /// Errors if there is nothing to commit.
    pub async fn commit_all(&self, message: impl Into<String>) -> Result<()> {
        let mut add = AddCommand::new();
        add.all();
        add.current_dir(self.repo.path());
        add.execute().await?;

        let mut commit = CommitCommand::new();
        commit.message(message);
        commit.current_dir(self.repo.path());
        commit.execute().await?;
        Ok(())
    }

    /// Bring the current branch up to date with its upstream via rebase.
    ///
    /// Equivalent to `git pull --rebase`. Errors if no upstream is configured
    /// or the rebase has conflicts.
    pub async fn sync(&self) -> Result<()> {
        let mut cmd = PullCommand::new();
        cmd.rebase();
        cmd.current_dir(self.repo.path());
        cmd.execute().await?;
        Ok(())
    }

    /// Squash-merge `branch` into the current branch.
    ///
    /// Equivalent to `git merge --squash <branch>`. The changes are staged
    /// but **not** committed — callers should follow up with
    /// [`commit_all`](Self::commit_all) or a raw commit to record them.
    pub async fn squash_merge(&self, branch: impl Into<String>) -> Result<()> {
        let mut cmd = MergeCommand::new();
        cmd.squash().commit_ref(branch);
        cmd.current_dir(self.repo.path());
        cmd.execute().await?;
        Ok(())
    }
}

impl Repository {
    /// High-level workflow compositions.
    #[must_use]
    pub fn workflow(&self) -> WorkflowOps<'_> {
        WorkflowOps { repo: self }
    }
}
