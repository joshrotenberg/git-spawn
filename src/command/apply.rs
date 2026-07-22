//! `git apply` — apply a patch to the working tree and/or index.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// Builder for `git apply`.
///
/// Applies one or more patch files. Reading a patch from stdin is not modelled:
/// this builder always passes patch paths on the command line.
#[derive(Debug, Clone, Default)]
pub struct ApplyCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The patch files to apply, in the order they were added.
    pub patches: Vec<PathBuf>,
    /// `--check`: report whether the patch applies without changing anything.
    pub check: bool,
    /// `--reverse`: apply the patch backwards.
    pub reverse: bool,
    /// `--3way`: fall back to a three-way merge when the patch does not apply cleanly.
    pub three_way: bool,
    /// `--index`: apply to the index as well as the working tree.
    pub index: bool,
    /// `--cached`: apply to the index only, leaving the working tree alone.
    pub cached: bool,
    /// `-p<n>`: number of leading path components to strip.
    pub strip: Option<u32>,
}

impl ApplyCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply the patch at `path`. Call repeatedly to apply several patches.
    pub fn patch(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.patches.push(path.into());
        self
    }

    /// Check whether the patch applies, without touching the working tree or index.
    pub fn check(&mut self) -> &mut Self {
        self.check = true;
        self
    }

    /// Apply the patch in reverse.
    pub fn reverse(&mut self) -> &mut Self {
        self.reverse = true;
        self
    }

    /// Fall back to a three-way merge when the patch does not apply cleanly.
    pub fn three_way(&mut self) -> &mut Self {
        self.three_way = true;
        self
    }

    /// Apply to the index as well as the working tree.
    pub fn index(&mut self) -> &mut Self {
        self.index = true;
        self
    }

    /// Apply to the index only, leaving the working tree alone.
    pub fn cached(&mut self) -> &mut Self {
        self.cached = true;
        self
    }

    /// Strip `n` leading path components from every path in the patch.
    pub fn strip(&mut self, n: u32) -> &mut Self {
        self.strip = Some(n);
        self
    }
}

#[async_trait]
impl GitCommand for ApplyCommand {
    /// Raw output. `git apply` prints nothing on success and reports failures
    /// on stderr with a non-zero exit status.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["apply".to_string()];
        if self.check {
            args.push("--check".into());
        }
        if self.reverse {
            args.push("--reverse".into());
        }
        if self.three_way {
            args.push("--3way".into());
        }
        if self.index {
            args.push("--index".into());
        }
        if self.cached {
            args.push("--cached".into());
        }
        if let Some(n) = self.strip {
            args.push(format!("-p{n}"));
        }
        for patch in &self.patches {
            args.push(patch.display().to_string());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.patches.is_empty() {
            return Err(Error::invalid_config("apply requires at least one patch"));
        }
        self.execute_raw().await
    }
}
