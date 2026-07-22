//! `git cherry` — find commits not yet applied upstream.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git cherry`.
///
/// Compares the commits reachable from `head` against those reachable from
/// `upstream` and reports each one with a `+` (not applied upstream) or `-`
/// (an equivalent patch is already upstream) marker. The three revision
/// arguments are positional, so `head` requires `upstream` and `limit`
/// requires `head`; supplying one without its predecessor is rejected by
/// [`execute`](GitCommand::execute) rather than silently shifting positions.
///
/// With no arguments at all, git falls back to the current branch's
/// configured upstream, and fails when there is none.
///
/// Output is left as a [`CommandOutput`]; [`parse_entries`](Self::parse_entries)
/// turns it into typed [`CherryEntry`](crate::parse::CherryEntry) values.
#[derive(Debug, Clone, Default)]
pub struct CherryCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The upstream branch to compare against.
    pub upstream: Option<String>,
    /// The working branch, defaulting to `HEAD` inside git.
    pub head: Option<String>,
    /// Only consider commits reachable from `head` but not from this limit.
    pub limit: Option<String>,
    /// `-v`: append each commit's subject line to its entry.
    pub verbose: bool,
}

impl CherryCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compare against `upstream`.
    pub fn upstream(&mut self, upstream: impl Into<String>) -> &mut Self {
        self.upstream = Some(upstream.into());
        self
    }

    /// Use `head` as the working branch instead of `HEAD`.
    pub fn head(&mut self, head: impl Into<String>) -> &mut Self {
        self.head = Some(head.into());
        self
    }

    /// Skip commits reachable from `limit`.
    pub fn limit(&mut self, limit: impl Into<String>) -> &mut Self {
        self.limit = Some(limit.into());
        self
    }

    /// Include each commit's subject line in the output.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }

    /// Parse a completed run's [`CommandOutput`] into typed entries.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_entries(&self, output: &CommandOutput) -> Vec<crate::parse::CherryEntry> {
        crate::parse::parse_cherry(&output.stdout_str())
    }
}

#[async_trait]
impl GitCommand for CherryCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["cherry".to_string()];
        if self.verbose {
            args.push("-v".into());
        }
        if let Some(upstream) = &self.upstream {
            args.push(upstream.clone());
        }
        if let Some(head) = &self.head {
            args.push(head.clone());
        }
        if let Some(limit) = &self.limit {
            args.push(limit.clone());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.head.is_some() && self.upstream.is_none() {
            return Err(Error::invalid_config(
                "cherry: head requires an upstream, the arguments are positional",
            ));
        }
        if self.limit.is_some() && self.head.is_none() {
            return Err(Error::invalid_config(
                "cherry: limit requires a head, the arguments are positional",
            ));
        }
        self.execute_raw().await
    }
}
