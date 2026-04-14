//! `git log` — show commit logs.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git log`.
#[derive(Debug, Clone, Default)]
pub struct LogCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `-n`.
    pub max_count: Option<u32>,
    /// `--skip`.
    pub skip: Option<u32>,
    /// `--oneline`.
    pub oneline: bool,
    /// `--graph`.
    pub graph: bool,
    /// `--all`.
    pub all: bool,
    /// `--reverse`.
    pub reverse: bool,
    /// `--format` / `--pretty=format:`.
    pub format: Option<String>,
    /// `--since`.
    pub since: Option<String>,
    /// `--until`.
    pub until: Option<String>,
    /// `--author`.
    pub author: Option<String>,
    /// `--grep`.
    pub grep: Option<String>,
    /// Revision range (e.g. `HEAD~5..HEAD`) or refs.
    pub revisions: Vec<String>,
    /// Pathspec filters.
    pub paths: Vec<String>,
}

impl LogCommand {
    /// New `log` builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Limit to `n` commits.
    pub fn max_count(&mut self, n: u32) -> &mut Self {
        self.max_count = Some(n);
        self
    }

    /// Skip first `n` commits.
    pub fn skip(&mut self, n: u32) -> &mut Self {
        self.skip = Some(n);
        self
    }

    /// `--oneline`.
    pub fn oneline(&mut self) -> &mut Self {
        self.oneline = true;
        self
    }

    /// `--graph`.
    pub fn graph(&mut self) -> &mut Self {
        self.graph = true;
        self
    }

    /// `--all`.
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }

    /// `--reverse`.
    pub fn reverse(&mut self) -> &mut Self {
        self.reverse = true;
        self
    }

    /// Set a `--format=...` template.
    pub fn format(&mut self, fmt: impl Into<String>) -> &mut Self {
        self.format = Some(fmt.into());
        self
    }

    /// Filter by `--since`.
    pub fn since(&mut self, s: impl Into<String>) -> &mut Self {
        self.since = Some(s.into());
        self
    }

    /// Filter by `--until`.
    pub fn until(&mut self, s: impl Into<String>) -> &mut Self {
        self.until = Some(s.into());
        self
    }

    /// Filter by author.
    pub fn author(&mut self, s: impl Into<String>) -> &mut Self {
        self.author = Some(s.into());
        self
    }

    /// Filter by commit-message grep.
    pub fn grep(&mut self, s: impl Into<String>) -> &mut Self {
        self.grep = Some(s.into());
        self
    }

    /// Add a revision/range/ref.
    pub fn revision(&mut self, r: impl Into<String>) -> &mut Self {
        self.revisions.push(r.into());
        self
    }

    /// Filter by path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }
}

#[async_trait]
impl GitCommand for LogCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["log".to_string()];
        if let Some(n) = self.max_count {
            args.push(format!("-n{n}"));
        }
        if let Some(n) = self.skip {
            args.push(format!("--skip={n}"));
        }
        if self.oneline {
            args.push("--oneline".into());
        }
        if self.graph {
            args.push("--graph".into());
        }
        if self.all {
            args.push("--all".into());
        }
        if self.reverse {
            args.push("--reverse".into());
        }
        if let Some(f) = &self.format {
            args.push(format!("--format={f}"));
        }
        if let Some(s) = &self.since {
            args.push(format!("--since={s}"));
        }
        if let Some(s) = &self.until {
            args.push(format!("--until={s}"));
        }
        if let Some(s) = &self.author {
            args.push(format!("--author={s}"));
        }
        if let Some(s) = &self.grep {
            args.push(format!("--grep={s}"));
        }
        args.extend(self.revisions.iter().cloned());
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
