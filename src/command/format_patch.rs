//! `git format-patch` — prepare commits as mbox-style patch files.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// Builder for `git format-patch`.
///
/// Writes one patch file per commit in the given revision range and returns the
/// paths git reported on stdout. `--stdout` is deliberately not modelled here:
/// it replaces the path listing with the patch bodies themselves, which does not
/// fit this command's [`Output`](GitCommand::Output).
#[derive(Debug, Clone, Default)]
pub struct FormatPatchCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The revision range to format (e.g. `"HEAD~3..HEAD"`).
    pub rev_spec: Option<String>,
    /// `-o <dir>`: write patches into this directory instead of the cwd.
    pub output_dir: Option<PathBuf>,
    /// `-n`: force numbered `[PATCH n/m]` subject prefixes.
    pub numbered: bool,
    /// `--signoff`: add a `Signed-off-by` trailer to each patch.
    pub signoff: bool,
}

impl FormatPatchCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Format the commits selected by `rev_spec`.
    pub fn rev_spec(&mut self, spec: impl Into<String>) -> &mut Self {
        self.rev_spec = Some(spec.into());
        self
    }

    /// Write the generated patches into `dir`.
    pub fn output_dir(&mut self, dir: impl Into<PathBuf>) -> &mut Self {
        self.output_dir = Some(dir.into());
        self
    }

    /// Force numbered subject prefixes even for a single patch.
    pub fn numbered(&mut self) -> &mut Self {
        self.numbered = true;
        self
    }

    /// Add a `Signed-off-by` trailer to each generated patch.
    pub fn signoff(&mut self) -> &mut Self {
        self.signoff = true;
        self
    }
}

#[async_trait]
impl GitCommand for FormatPatchCommand {
    /// The paths of the generated patch files, in the order git printed them.
    type Output = Vec<PathBuf>;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["format-patch".to_string()];
        if self.numbered {
            args.push("-n".into());
        }
        if self.signoff {
            args.push("--signoff".into());
        }
        if let Some(dir) = &self.output_dir {
            args.push("-o".into());
            args.push(dir.display().to_string());
        }
        if let Some(spec) = &self.rev_spec {
            args.push(spec.clone());
        }
        args
    }

    async fn execute(&self) -> Result<Vec<PathBuf>> {
        if self.rev_spec.is_none() {
            return Err(Error::invalid_config(
                "format-patch requires a revision range",
            ));
        }
        let out = self.execute_raw().await?;
        Ok(out.stdout_lines().into_iter().map(PathBuf::from).collect())
    }
}
