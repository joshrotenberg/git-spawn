//! Walk a repository's commit history into typed [`Commit`] structs.
//!
//! Reached through [`Repository::history`], which returns a [`HistoryWalk`]
//! builder. Configure with `revision`, `max_count`, `since`, `author`, etc.,
//! then call [`HistoryWalk::execute`] to spawn `git log` with a stable
//! `--format` and parse the output into `Vec<Commit>`.
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Last 20 commits authored by Alice.
//! let commits = repo
//!     .history()
//!     .max_count(20)
//!     .author("Alice")
//!     .execute()
//!     .await?;
//! for c in commits {
//!     println!("{} {} {}", c.short_sha, c.author_name, c.subject);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Parsing reuses the [`crate::parse::parse_log`] machinery and the
//! [`crate::parse::LOG_FORMAT`] token string. [`Commit`] is a re-export of
//! [`crate::parse::CommitEntry`] under a friendlier name for the workflow API.

use crate::command::GitCommand;
use crate::command::log::LogCommand;
use crate::error::Result;
use crate::parse::{LOG_FORMAT, parse_log};
use crate::repo::Repository;

pub use crate::parse::CommitEntry as Commit;

/// Builder for one history walk. Configure with the chained setters, then
/// call [`execute`](Self::execute).
#[derive(Debug)]
pub struct HistoryWalk<'a> {
    repo: &'a Repository,
    revisions: Vec<String>,
    paths: Vec<String>,
    max_count: Option<u32>,
    skip: Option<u32>,
    since: Option<String>,
    until: Option<String>,
    author: Option<String>,
    grep: Option<String>,
    reverse: bool,
}

impl<'a> HistoryWalk<'a> {
    fn new(repo: &'a Repository) -> Self {
        Self {
            repo,
            revisions: Vec::new(),
            paths: Vec::new(),
            max_count: None,
            skip: None,
            since: None,
            until: None,
            author: None,
            grep: None,
            reverse: false,
        }
    }

    /// Limit results to the most recent `n` commits.
    #[must_use]
    pub fn max_count(mut self, n: u32) -> Self {
        self.max_count = Some(n);
        self
    }

    /// Skip the first `n` commits before collecting.
    #[must_use]
    pub fn skip(mut self, n: u32) -> Self {
        self.skip = Some(n);
        self
    }

    /// Filter by `--since` (any value `git log` accepts, e.g. `"2.weeks.ago"`).
    #[must_use]
    pub fn since(mut self, s: impl Into<String>) -> Self {
        self.since = Some(s.into());
        self
    }

    /// Filter by `--until`.
    #[must_use]
    pub fn until(mut self, s: impl Into<String>) -> Self {
        self.until = Some(s.into());
        self
    }

    /// Filter by author (`--author`).
    #[must_use]
    pub fn author(mut self, s: impl Into<String>) -> Self {
        self.author = Some(s.into());
        self
    }

    /// Filter by commit-message grep (`--grep`).
    #[must_use]
    pub fn grep(mut self, s: impl Into<String>) -> Self {
        self.grep = Some(s.into());
        self
    }

    /// Add a revision, range, or ref (e.g. `"HEAD~10..HEAD"`).
    /// Multiple calls accumulate.
    #[must_use]
    pub fn revision(mut self, r: impl Into<String>) -> Self {
        self.revisions.push(r.into());
        self
    }

    /// Restrict to commits touching `path`. Multiple calls accumulate.
    #[must_use]
    pub fn path(mut self, p: impl Into<String>) -> Self {
        self.paths.push(p.into());
        self
    }

    /// Reverse the output order (`--reverse`).
    #[must_use]
    pub fn reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Spawn `git log` with the configured filters and parse the result.
    pub async fn execute(self) -> Result<Vec<Commit>> {
        let mut cmd = LogCommand::new();
        cmd.format(LOG_FORMAT);
        if let Some(n) = self.max_count {
            cmd.max_count(n);
        }
        if let Some(n) = self.skip {
            cmd.skip(n);
        }
        if let Some(s) = self.since {
            cmd.since(s);
        }
        if let Some(s) = self.until {
            cmd.until(s);
        }
        if let Some(s) = self.author {
            cmd.author(s);
        }
        if let Some(s) = self.grep {
            cmd.grep(s);
        }
        if self.reverse {
            cmd.reverse();
        }
        for r in self.revisions {
            cmd.revision(r);
        }
        for p in self.paths {
            cmd.path(p);
        }
        cmd.current_dir(self.repo.path());
        let out = cmd.execute().await?;
        parse_log(&out.stdout_str())
    }
}

impl Repository {
    /// Walk commit history with a chained-builder filter.
    #[must_use]
    pub fn history(&self) -> HistoryWalk<'_> {
        HistoryWalk::new(self)
    }
}
