//! `git cat-file` — provide content or type/size information for repository objects.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Mode of operation for `cat-file`.
#[derive(Debug, Clone, Copy)]
pub enum CatFileMode {
    /// `-t`: print the object's type.
    Type,
    /// `-s`: print the object's size.
    Size,
    /// `-e`: exit 0 if object exists, non-zero otherwise.
    Exists,
    /// `-p`: pretty-print the object's contents.
    PrettyPrint,
}

/// Builder for `git cat-file`.
#[derive(Debug, Clone)]
pub struct CatFileCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Operation mode.
    pub mode: CatFileMode,
    /// Object to inspect.
    pub object: String,
}

impl CatFileCommand {
    /// Create a `cat-file -p <object>` command.
    pub fn pretty_print(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::PrettyPrint,
            object: object.into(),
        }
    }

    /// Create a `cat-file -t <object>` command.
    pub fn object_type(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::Type,
            object: object.into(),
        }
    }

    /// Create a `cat-file -s <object>` command.
    pub fn size(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::Size,
            object: object.into(),
        }
    }

    /// Create a `cat-file -e <object>` command.
    pub fn exists(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::Exists,
            object: object.into(),
        }
    }
}

#[async_trait]
impl GitCommand for CatFileCommand {
    /// Trimmed stdout. For `Exists` mode, success is reported via `Ok(String::new())`;
    /// a missing object returns [`Error::CommandFailed`].
    type Output = String;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let flag = match self.mode {
            CatFileMode::Type => "-t",
            CatFileMode::Size => "-s",
            CatFileMode::Exists => "-e",
            CatFileMode::PrettyPrint => "-p",
        };
        vec!["cat-file".into(), flag.into(), self.object.clone()]
    }
    async fn execute(&self) -> Result<String> {
        if self.object.is_empty() {
            return Err(Error::invalid_config(
                "cat-file requires a non-empty object",
            ));
        }
        let out = self.execute_raw().await?;
        Ok(out.stdout_trimmed().to_string())
    }
}
