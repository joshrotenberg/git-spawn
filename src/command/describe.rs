//! `git describe` — describe a commit using the most recent reachable tag.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git describe`.
#[derive(Debug, Clone, Default)]
pub struct DescribeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Commit-ish(es) to describe.
    pub commits: Vec<String>,
    /// `--tags` include lightweight tags.
    pub tags: bool,
    /// `--all` include all refs, not just tags.
    pub all: bool,
    /// `--always` fall back to abbreviated SHA when nothing matches.
    pub always: bool,
    /// `--long` always show the long form (`tag-count-gSHA`).
    pub long: bool,
    /// `--dirty[=<mark>]`.
    pub dirty: Option<Option<String>>,
    /// `--abbrev=N`.
    pub abbrev: Option<u32>,
    /// `--match=<pattern>`.
    pub match_pattern: Option<String>,
    /// `--exclude=<pattern>`.
    pub exclude: Option<String>,
    /// `--first-parent`.
    pub first_parent: bool,
}

impl DescribeCommand {
    /// New describe command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a commit-ish to describe.
    pub fn commit(&mut self, c: impl Into<String>) -> &mut Self {
        self.commits.push(c.into());
        self
    }

    /// `--tags`.
    pub fn tags(&mut self) -> &mut Self {
        self.tags = true;
        self
    }

    /// `--all`.
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }

    /// `--always`.
    pub fn always(&mut self) -> &mut Self {
        self.always = true;
        self
    }

    /// `--long`.
    pub fn long(&mut self) -> &mut Self {
        self.long = true;
        self
    }

    /// `--dirty` with default mark (`-dirty`).
    pub fn dirty(&mut self) -> &mut Self {
        self.dirty = Some(None);
        self
    }

    /// `--dirty=<mark>`.
    pub fn dirty_mark(&mut self, mark: impl Into<String>) -> &mut Self {
        self.dirty = Some(Some(mark.into()));
        self
    }

    /// `--abbrev=N`.
    pub fn abbrev(&mut self, n: u32) -> &mut Self {
        self.abbrev = Some(n);
        self
    }

    /// `--match=<pattern>`.
    pub fn match_pattern(&mut self, p: impl Into<String>) -> &mut Self {
        self.match_pattern = Some(p.into());
        self
    }

    /// `--exclude=<pattern>`.
    pub fn exclude(&mut self, p: impl Into<String>) -> &mut Self {
        self.exclude = Some(p.into());
        self
    }

    /// `--first-parent`.
    pub fn first_parent(&mut self) -> &mut Self {
        self.first_parent = true;
        self
    }
}

#[async_trait]
impl GitCommand for DescribeCommand {
    /// Trimmed description string, e.g. `v1.2.3-5-gabc1234`.
    type Output = String;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["describe".to_string()];
        if self.tags {
            args.push("--tags".into());
        }
        if self.all {
            args.push("--all".into());
        }
        if self.always {
            args.push("--always".into());
        }
        if self.long {
            args.push("--long".into());
        }
        if self.first_parent {
            args.push("--first-parent".into());
        }
        match &self.dirty {
            Some(None) => args.push("--dirty".into()),
            Some(Some(mark)) => args.push(format!("--dirty={mark}")),
            None => {}
        }
        if let Some(n) = self.abbrev {
            args.push(format!("--abbrev={n}"));
        }
        if let Some(p) = &self.match_pattern {
            args.push(format!("--match={p}"));
        }
        if let Some(p) = &self.exclude {
            args.push(format!("--exclude={p}"));
        }
        args.extend(self.commits.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<String> {
        let out = self.execute_raw().await?;
        Ok(out.stdout_trimmed())
    }
}
