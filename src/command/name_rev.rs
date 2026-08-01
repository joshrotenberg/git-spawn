//! `git name-rev` — find symbolic names for revisions.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git name-rev`.
///
/// Names each given revision after the ref it is reachable from, one
/// `<sha> <name>` line per revision, where the name carries the distance back
/// from the ref's tip: `main~2`, `tags/v1.0^0`. A revision no ref reaches is
/// named `undefined`.
///
/// [`name_only`](Self::name_only) drops the SHA column,
/// [`tags`](Self::tags) restricts the naming to `refs/tags/`, and
/// [`refs`](Self::refs) restricts it to refs matching a shell pattern and may
/// be repeated.
///
/// The revisions are positional and git needs at least one of them, so
/// [`execute`](GitCommand::execute) rejects an empty command as
/// [`Error::InvalidConfig`] rather than letting git fail on a usage error.
///
/// `--all`, `--annotate-stdin`, `--no-undefined`, `--always` and `--exclude`
/// are left to the documented raw-argument escape hatch.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct NameRevCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The positional revisions to name, in the order they were added.
    pub revs: Vec<String>,
    /// `--name-only`: print the name alone, without the input revision.
    pub name_only: bool,
    /// `--tags`: name using tags only, not every ref.
    pub tags: bool,
    /// `--refs=<pattern>`: name using only refs matching these patterns.
    pub refs: Vec<String>,
}

impl NameRevCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one positional revision to name.
    pub fn rev(&mut self, rev: impl Into<String>) -> &mut Self {
        self.revs.push(rev.into());
        self
    }

    /// Append several positional revisions to name.
    pub fn revs<I, S>(&mut self, revs: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.revs.extend(revs.into_iter().map(Into::into));
        self
    }

    /// Print only the name, dropping the input revision from each line.
    pub fn name_only(&mut self) -> &mut Self {
        self.name_only = true;
        self
    }

    /// Name revisions using tags only.
    pub fn tags(&mut self) -> &mut Self {
        self.tags = true;
        self
    }

    /// Name revisions using only refs matching `pattern`.
    ///
    /// May be called more than once; git accepts the flag repeatedly and takes
    /// the union of the patterns.
    pub fn refs(&mut self, pattern: impl Into<String>) -> &mut Self {
        self.refs.push(pattern.into());
        self
    }
}

#[async_trait]
impl GitCommand for NameRevCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["name-rev".to_string()];
        if self.name_only {
            args.push("--name-only".into());
        }
        if self.tags {
            args.push("--tags".into());
        }
        for pattern in &self.refs {
            args.push(format!("--refs={pattern}"));
        }
        args.extend(self.revs.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.revs.is_empty() {
            return Err(Error::invalid_config(
                "name-rev requires at least one revision",
            ));
        }
        self.execute_raw().await
    }
}
