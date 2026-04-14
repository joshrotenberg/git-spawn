//! `git init` — create an empty Git repository or reinitialize an existing one.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::Result;
use crate::repo::Repository;
use async_trait::async_trait;
use std::path::PathBuf;

/// Builder for `git init`.
#[derive(Debug, Clone, Default)]
pub struct InitCommand {
    /// Shared executor (args, cwd, env, timeout).
    pub executor: CommandExecutor,
    /// Directory to initialize. If `None`, uses the executor's cwd.
    pub directory: Option<PathBuf>,
    /// Create a bare repository (`--bare`).
    pub bare: bool,
    /// Suppress output (`-q`, `--quiet`).
    pub quiet: bool,
    /// Initial branch name (`--initial-branch`).
    pub initial_branch: Option<String>,
    /// Shared repository mode (`--shared`).
    pub shared: Option<String>,
    /// Template directory (`--template`).
    pub template: Option<PathBuf>,
    /// Separate git dir (`--separate-git-dir`).
    pub separate_git_dir: Option<PathBuf>,
}

impl InitCommand {
    /// Build an `init` with no directory argument (uses cwd).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an `init` for the given directory.
    #[must_use]
    pub fn in_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            directory: Some(path.into()),
            ..Self::default()
        }
    }

    /// Create a bare repository.
    pub fn bare(&mut self) -> &mut Self {
        self.bare = true;
        self
    }

    /// Suppress stdout.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }

    /// Set the initial branch name (default on modern git: `main`).
    pub fn initial_branch(&mut self, name: impl Into<String>) -> &mut Self {
        self.initial_branch = Some(name.into());
        self
    }

    /// Enable sharing mode (e.g. `"group"`, `"all"`, `"0660"`).
    pub fn shared(&mut self, mode: impl Into<String>) -> &mut Self {
        self.shared = Some(mode.into());
        self
    }

    /// Use a template directory.
    pub fn template(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.template = Some(path.into());
        self
    }

    /// Store the git dir separately from the working tree.
    pub fn separate_git_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.separate_git_dir = Some(path.into());
        self
    }
}

#[async_trait]
impl GitCommand for InitCommand {
    type Output = Repository;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["init".to_string()];
        if self.bare {
            args.push("--bare".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if let Some(branch) = &self.initial_branch {
            args.push(format!("--initial-branch={branch}"));
        }
        if let Some(mode) = &self.shared {
            args.push(format!("--shared={mode}"));
        }
        if let Some(t) = &self.template {
            args.push(format!("--template={}", t.display()));
        }
        if let Some(g) = &self.separate_git_dir {
            args.push(format!("--separate-git-dir={}", g.display()));
        }
        if let Some(d) = &self.directory {
            args.push(d.display().to_string());
        }
        args
    }

    async fn execute(&self) -> Result<Repository> {
        self.execute_raw().await?;
        let path = self
            .directory
            .clone()
            .or_else(|| self.executor.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        Ok(Repository::new_unchecked(path))
    }
}
