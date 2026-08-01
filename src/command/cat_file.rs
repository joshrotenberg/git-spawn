//! `git cat-file` — provide content or type/size information for repository objects.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Mode of operation for `cat-file`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum CatFileMode {
    /// `-t`: print the object's type.
    Type,
    /// `-s`: print the object's size.
    Size,
    /// `-e`: exit 0 if object exists, non-zero otherwise.
    Exists,
    /// `-p`: pretty-print the object's contents.
    PrettyPrint,
    /// `<type> <object>`: print the object's contents after verifying its type.
    TypeChecked,
}

/// Builder for `git cat-file`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CatFileCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Operation mode.
    pub mode: CatFileMode,
    /// Object to inspect.
    pub object: String,
    /// Required object type for [`CatFileMode::TypeChecked`].
    pub expected_type: Option<String>,
}

impl CatFileCommand {
    /// Create a `cat-file -p <object>` command.
    pub fn pretty_print(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::PrettyPrint,
            object: object.into(),
            expected_type: None,
        }
    }

    /// Create a `cat-file -t <object>` command.
    pub fn object_type(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::Type,
            object: object.into(),
            expected_type: None,
        }
    }

    /// Create a `cat-file -s <object>` command.
    pub fn size(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::Size,
            object: object.into(),
            expected_type: None,
        }
    }

    /// Create a `cat-file -e <object>` command.
    pub fn exists(object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::Exists,
            object: object.into(),
            expected_type: None,
        }
    }

    /// Create a `cat-file <type> <object>` command.
    ///
    /// Git prints the object's contents only after verifying that the object
    /// has the requested type.
    pub fn type_checked(expected_type: impl Into<String>, object: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            mode: CatFileMode::TypeChecked,
            object: object.into(),
            expected_type: Some(expected_type.into()),
        }
    }

    fn validate(&self) -> Result<()> {
        if matches!(self.mode, CatFileMode::TypeChecked)
            && self.expected_type.as_deref().is_none_or(str::is_empty)
        {
            return Err(Error::invalid_config(
                "cat-file requires a non-empty expected type",
            ));
        }
        if self.object.is_empty() {
            return Err(Error::invalid_config(
                "cat-file requires a non-empty object",
            ));
        }
        Ok(())
    }

    /// Run the command and return stdout as raw, untrimmed bytes.
    ///
    /// Prefer this over [`execute`](GitCommand::execute) when the object may be
    /// binary (typically `pretty_print` on a blob): `execute` decodes stdout
    /// lossily as UTF-8 and trims trailing whitespace, either of which corrupts
    /// binary content.
    pub async fn execute_bytes(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let out = self.execute_raw().await?;
        Ok(out.stdout)
    }
}

#[async_trait]
impl GitCommand for CatFileCommand {
    /// Trimmed, lossily-decoded stdout. For `Exists` mode, success is reported
    /// via `Ok(String::new())`; a missing object returns
    /// [`Error::CommandFailed`]. For binary blobs use
    /// [`execute_bytes`](CatFileCommand::execute_bytes) instead.
    type Output = String;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mode = match self.mode {
            CatFileMode::Type => "-t",
            CatFileMode::Size => "-s",
            CatFileMode::Exists => "-e",
            CatFileMode::PrettyPrint => "-p",
            CatFileMode::TypeChecked => self.expected_type.as_deref().unwrap_or_default(),
        };
        vec!["cat-file".into(), mode.into(), self.object.clone()]
    }
    async fn execute(&self) -> Result<String> {
        self.validate()?;
        let out = self.execute_raw().await?;
        Ok(out.stdout_trimmed())
    }
}
