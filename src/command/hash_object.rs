//! `git hash-object` — compute object ID and optionally create a blob from a file.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// Builder for `git hash-object`.
#[derive(Debug, Clone, Default)]
pub struct HashObjectCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Path(s) to hash.
    pub paths: Vec<PathBuf>,
    /// `-w`: also write the object into the object database.
    pub write: bool,
    /// `--stdin`: read from stdin. (Currently unsupported in this wrapper; use
    /// `path()` on a file containing the desired bytes.)
    pub stdin: bool,
    /// `-t <type>`: override the object type.
    pub object_type: Option<String>,
}

impl HashObjectCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Hash the file at `path`.
    pub fn path(&mut self, p: impl Into<PathBuf>) -> &mut Self {
        self.paths.push(p.into());
        self
    }

    /// Also write the resulting object into `.git/objects`.
    pub fn write(&mut self) -> &mut Self {
        self.write = true;
        self
    }

    /// Override the object type (e.g. `"tree"`, `"commit"`, `"tag"`).
    pub fn object_type(&mut self, t: impl Into<String>) -> &mut Self {
        self.object_type = Some(t.into());
        self
    }
}

#[async_trait]
impl GitCommand for HashObjectCommand {
    /// The computed SHA(s). When multiple paths are provided, one SHA per line.
    type Output = String;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["hash-object".to_string()];
        if self.write {
            args.push("-w".into());
        }
        if let Some(t) = &self.object_type {
            args.push("-t".into());
            args.push(t.clone());
        }
        if self.stdin {
            args.push("--stdin".into());
        }
        args.extend(self.paths.iter().map(|p| p.display().to_string()));
        args
    }

    async fn execute(&self) -> Result<String> {
        if self.paths.is_empty() && !self.stdin {
            return Err(Error::invalid_config(
                "hash-object requires at least one path",
            ));
        }
        let out = self.execute_raw().await?;
        Ok(out.stdout_trimmed())
    }
}
