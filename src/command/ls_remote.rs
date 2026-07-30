//! `git ls-remote` — list references in a remote repository.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git ls-remote`.
///
/// Queries a remote for its refs without fetching any objects. The remote may
/// be a configured remote name, a URL, or a local path; with none set git
/// falls back to the current branch's configured remote and fails when there
/// is none.
///
/// The repository and the patterns are positional, and git takes the first
/// non-option argument as the repository. Patterns without a repository would
/// therefore make the first pattern the remote, so
/// [`execute`](GitCommand::execute) rejects that combination rather than
/// querying the wrong thing.
///
/// Output is left as a [`CommandOutput`]; [`parse_entries`](Self::parse_entries)
/// turns it into typed [`LsRemoteEntry`](crate::parse::LsRemoteEntry) values.
#[derive(Debug, Clone, Default)]
pub struct LsRemoteCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The remote to query: a remote name, a URL, or a path.
    pub repository: Option<String>,
    /// Ref patterns limiting the output.
    pub patterns: Vec<String>,
    /// `--heads`: limit to `refs/heads/`.
    pub heads: bool,
    /// `--tags`: limit to `refs/tags/`.
    pub tags: bool,
    /// `--refs`: drop `HEAD` and the peeled `^{}` lines.
    pub refs: bool,
    /// `--symref`: also report what the symbolic refs point at.
    pub symref: bool,
    /// `--exit-code`: exit 2 when no ref matched.
    pub exit_code: bool,
    /// `-q`: suppress the `From <url>` progress line on stderr.
    pub quiet: bool,
}

impl LsRemoteCommand {
    /// New command, querying the configured remote.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Query `repository`: a remote name, a URL, or a path.
    #[must_use]
    pub fn remote(repository: impl Into<String>) -> Self {
        Self {
            repository: Some(repository.into()),
            ..Self::default()
        }
    }

    /// Set the remote to query.
    pub fn repository(&mut self, repository: impl Into<String>) -> &mut Self {
        self.repository = Some(repository.into());
        self
    }

    /// Add a ref pattern. Requires a repository, since the two are positional.
    pub fn pattern(&mut self, pattern: impl Into<String>) -> &mut Self {
        self.patterns.push(pattern.into());
        self
    }

    /// Limit the output to branches.
    pub fn heads(&mut self) -> &mut Self {
        self.heads = true;
        self
    }

    /// Limit the output to tags.
    pub fn tags(&mut self) -> &mut Self {
        self.tags = true;
        self
    }

    /// Drop `HEAD` and the peeled `^{}` lines (`--refs`).
    pub fn refs(&mut self) -> &mut Self {
        self.refs = true;
        self
    }

    /// Report symbolic ref targets as well (`--symref`).
    pub fn symref(&mut self) -> &mut Self {
        self.symref = true;
        self
    }

    /// Exit with 2 when no ref matched (`--exit-code`).
    pub fn exit_code(&mut self) -> &mut Self {
        self.exit_code = true;
        self
    }

    /// Suppress the progress line on stderr (`-q`).
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }

    /// Parse a completed run's [`CommandOutput`] into typed entries.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_entries(&self, output: &CommandOutput) -> Vec<crate::parse::LsRemoteEntry> {
        crate::parse::parse_ls_remote(&output.stdout_str())
    }
}

#[async_trait]
impl GitCommand for LsRemoteCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["ls-remote".to_string()];
        if self.heads {
            args.push("--heads".into());
        }
        if self.tags {
            args.push("--tags".into());
        }
        if self.refs {
            args.push("--refs".into());
        }
        if self.symref {
            args.push("--symref".into());
        }
        if self.exit_code {
            args.push("--exit-code".into());
        }
        if self.quiet {
            args.push("-q".into());
        }
        if let Some(repository) = &self.repository {
            args.push(repository.clone());
        }
        args.extend(self.patterns.iter().cloned());
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if !self.patterns.is_empty() && self.repository.is_none() {
            return Err(Error::invalid_config(
                "ls-remote: patterns require a repository, the arguments are positional",
            ));
        }
        self.execute_raw().await
    }
}
