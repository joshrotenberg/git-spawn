//! `git --version` — report the version of the git binary in use.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git --version`.
///
/// Unlike the rest of the wrappers this one drives a top-level flag rather
/// than a subcommand, so the built argv is just `--version`. The output is a
/// single `git version <version>` line, with a platform suffix on some builds
/// (`2.45.1.windows.1`, `2.39.5 (Apple Git-154)`).
///
/// [`build_options`](Self::build_options) adds `--build-options`, which keeps
/// the version line and appends the build's cpu, commit, and sizeof lines.
///
/// Output is left as a [`CommandOutput`]; [`parse_version`](Self::parse_version)
/// turns the version line into a typed
/// [`GitVersion`](crate::parse::GitVersion).
#[derive(Debug, Clone, Default)]
pub struct VersionCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `--build-options`: append the build's cpu, commit, and sizeof lines.
    pub build_options: bool,
}

impl VersionCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Report the build options alongside the version.
    pub fn build_options(&mut self) -> &mut Self {
        self.build_options = true;
        self
    }

    /// Parse a completed run's [`CommandOutput`] into a typed version.
    ///
    /// Returns [`None`] when the output carries no `git version` line, which
    /// is the only shape the parser accepts.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_version(&self, output: &CommandOutput) -> Option<crate::parse::GitVersion> {
        crate::parse::parse_version(&output.stdout_str())
    }
}

#[async_trait]
impl GitCommand for VersionCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["--version".to_string()];
        if self.build_options {
            args.push("--build-options".into());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
