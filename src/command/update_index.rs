//! `git update-index` — register file contents in the index.
use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
/// A cache entry supplied with `--cacheinfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheInfo {
    /// File mode.
    pub mode: String,
    /// Object ID.
    pub object: String,
    /// Index path.
    pub path: String,
}
/// Builder for `git update-index`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct UpdateIndexCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Permit adding files.
    pub add: bool,
    /// Remove missing files.
    pub remove: bool,
    /// Mark paths assume-unchanged.
    pub assume_unchanged: bool,
    /// Cache entries.
    pub cacheinfo: Vec<CacheInfo>,
    /// Paths to manipulate.
    pub paths: Vec<String>,
}
impl UpdateIndexCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Enable `--add`.
    pub fn add(&mut self) -> &mut Self {
        self.add = true;
        self
    }
    /// Enable `--remove`.
    pub fn remove(&mut self) -> &mut Self {
        self.remove = true;
        self
    }
    /// Enable `--assume-unchanged`.
    pub fn assume_unchanged(&mut self) -> &mut Self {
        self.assume_unchanged = true;
        self
    }
    /// Add a cache entry.
    pub fn cacheinfo(
        &mut self,
        mode: impl Into<String>,
        object: impl Into<String>,
        path: impl Into<String>,
    ) -> &mut Self {
        self.cacheinfo.push(CacheInfo {
            mode: mode.into(),
            object: object.into(),
            path: path.into(),
        });
        self
    }
    /// Add a path.
    pub fn path(&mut self, v: impl Into<String>) -> &mut Self {
        self.paths.push(v.into());
        self
    }
}
#[async_trait]
impl GitCommand for UpdateIndexCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["update-index".into()];
        if self.add {
            a.push("--add".into())
        }
        if self.remove {
            a.push("--remove".into())
        }
        if self.assume_unchanged {
            a.push("--assume-unchanged".into())
        }
        for c in &self.cacheinfo {
            a.push("--cacheinfo".into());
            a.push(c.mode.clone());
            a.push(c.object.clone());
            a.push(c.path.clone())
        }
        if !self.paths.is_empty() {
            a.push("--".into());
            a.extend(self.paths.iter().cloned())
        }
        a
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
