//! `git rev-list` — list commit objects in reverse chronological order.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git rev-list`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RevListCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `--max-count=<n>`.
    pub max_count: Option<usize>,
    /// `--since=<date>`.
    pub since: Option<String>,
    /// Print only the number of commits.
    pub count: bool,
    /// Revisions and revision ranges.
    pub revisions: Vec<String>,
}

impl RevListCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Limit the number of commits.
    pub fn max_count(&mut self, n: usize) -> &mut Self {
        self.max_count = Some(n);
        self
    }
    /// Include commits newer than the given date expression.
    pub fn since(&mut self, date: impl Into<String>) -> &mut Self {
        self.since = Some(date.into());
        self
    }
    /// Print only a count.
    pub fn count(&mut self) -> &mut Self {
        self.count = true;
        self
    }
    /// Add a revision or range (for example, `main..topic`).
    pub fn revision(&mut self, revision: impl Into<String>) -> &mut Self {
        self.revisions.push(revision.into());
        self
    }
    /// Add a revision range.
    pub fn range(&mut self, range: impl Into<String>) -> &mut Self {
        self.revision(range)
    }
}

#[async_trait]
impl GitCommand for RevListCommand {
    type Output = String;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["rev-list".into()];
        if let Some(n) = self.max_count {
            a.push(format!("--max-count={n}"));
        }
        if let Some(s) = &self.since {
            a.push(format!("--since={s}"));
        }
        if self.count {
            a.push("--count".into());
        }
        a.extend(self.revisions.iter().cloned());
        a
    }
    async fn execute(&self) -> Result<String> {
        Ok(self.execute_raw().await?.stdout_trimmed())
    }
}
