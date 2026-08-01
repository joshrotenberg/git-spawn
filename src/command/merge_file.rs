//! `git merge-file` — run a three-way file merge.
use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;
/// Builder for `git merge-file`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MergeFileCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Current file.
    pub current: Option<PathBuf>,
    /// Common ancestor file.
    pub base: Option<PathBuf>,
    /// Other file.
    pub other: Option<PathBuf>,
    /// Write result to stdout.
    pub stdout: bool,
    /// Use union conflict resolution.
    pub union: bool,
    /// Conflict marker size.
    pub marker_size: Option<usize>,
}
impl MergeFileCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the current file.
    pub fn current(&mut self, v: impl Into<PathBuf>) -> &mut Self {
        self.current = Some(v.into());
        self
    }
    /// Set the base file.
    pub fn base(&mut self, v: impl Into<PathBuf>) -> &mut Self {
        self.base = Some(v.into());
        self
    }
    /// Set the other file.
    pub fn other(&mut self, v: impl Into<PathBuf>) -> &mut Self {
        self.other = Some(v.into());
        self
    }
    /// Enable `--stdout`.
    pub fn stdout(&mut self) -> &mut Self {
        self.stdout = true;
        self
    }
    /// Enable `--union`.
    pub fn union(&mut self) -> &mut Self {
        self.union = true;
        self
    }
    /// Set conflict marker size.
    pub fn marker_size(&mut self, n: usize) -> &mut Self {
        self.marker_size = Some(n);
        self
    }
}
#[async_trait]
impl GitCommand for MergeFileCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        self.build_command_os_args()
            .iter()
            .map(|v| v.to_string_lossy().into_owned())
            .collect()
    }
    fn build_command_os_args(&self) -> Vec<std::ffi::OsString> {
        let mut a = vec!["merge-file".into()];
        if self.stdout {
            a.push("--stdout".into())
        }
        if self.union {
            a.push("--union".into())
        }
        if let Some(n) = self.marker_size {
            a.push(format!("--marker-size={n}").into())
        }
        if let Some(v) = &self.current {
            a.push(v.as_os_str().into())
        }
        if let Some(v) = &self.base {
            a.push(v.as_os_str().into())
        }
        if let Some(v) = &self.other {
            a.push(v.as_os_str().into())
        }
        a
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
