//! `git clone` — clone a repository into a new directory.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::Result;
use crate::repo::Repository;
use async_trait::async_trait;
use std::path::PathBuf;

/// Builder for `git clone`.
#[derive(Debug, Clone)]
pub struct CloneCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Remote URL (required).
    pub url: String,
    /// Local destination directory.
    pub directory: Option<PathBuf>,
    /// `--bare`.
    pub bare: bool,
    /// `--mirror`.
    pub mirror: bool,
    /// `--depth`.
    pub depth: Option<u32>,
    /// `--branch`.
    pub branch: Option<String>,
    /// `--single-branch`.
    pub single_branch: bool,
    /// `--recurse-submodules`.
    pub recurse_submodules: bool,
    /// `--origin`.
    pub origin: Option<String>,
    /// `--quiet`.
    pub quiet: bool,
}

impl CloneCommand {
    /// Create a new clone command for `url`.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            url: url.into(),
            directory: None,
            bare: false,
            mirror: false,
            depth: None,
            branch: None,
            single_branch: false,
            recurse_submodules: false,
            origin: None,
            quiet: false,
        }
    }

    /// Target directory.
    pub fn directory(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.directory = Some(path.into());
        self
    }

    /// Clone as a bare repository.
    pub fn bare(&mut self) -> &mut Self {
        self.bare = true;
        self
    }

    /// Mirror all refs.
    pub fn mirror(&mut self) -> &mut Self {
        self.mirror = true;
        self
    }

    /// Shallow clone with the given depth.
    pub fn depth(&mut self, depth: u32) -> &mut Self {
        self.depth = Some(depth);
        self
    }

    /// Check out the named branch instead of the remote HEAD.
    pub fn branch(&mut self, name: impl Into<String>) -> &mut Self {
        self.branch = Some(name.into());
        self
    }

    /// Clone only a single branch.
    pub fn single_branch(&mut self) -> &mut Self {
        self.single_branch = true;
        self
    }

    /// Recursively clone submodules.
    pub fn recurse_submodules(&mut self) -> &mut Self {
        self.recurse_submodules = true;
        self
    }

    /// Set the remote name (default `origin`).
    pub fn origin(&mut self, name: impl Into<String>) -> &mut Self {
        self.origin = Some(name.into());
        self
    }

    /// Suppress output.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
}

#[async_trait]
impl GitCommand for CloneCommand {
    type Output = Repository;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["clone".to_string()];
        if self.bare {
            args.push("--bare".into());
        }
        if self.mirror {
            args.push("--mirror".into());
        }
        if let Some(d) = self.depth {
            args.push(format!("--depth={d}"));
        }
        if let Some(b) = &self.branch {
            args.push("--branch".into());
            args.push(b.clone());
        }
        if self.single_branch {
            args.push("--single-branch".into());
        }
        if self.recurse_submodules {
            args.push("--recurse-submodules".into());
        }
        if let Some(o) = &self.origin {
            args.push("--origin".into());
            args.push(o.clone());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        args.push(self.url.clone());
        if let Some(d) = &self.directory {
            args.push(d.display().to_string());
        }
        args
    }

    async fn execute(&self) -> Result<Repository> {
        self.execute_raw().await?;
        let dir = self
            .directory
            .clone()
            .unwrap_or_else(|| PathBuf::from(infer_dest_dir(&self.url)));
        let full = if dir.is_absolute() {
            dir
        } else {
            self.executor
                .cwd
                .clone()
                .map_or_else(|| dir.clone(), |c| c.join(&dir))
        };
        Ok(Repository::new_unchecked(full))
    }
}

fn infer_dest_dir(url: &str) -> String {
    let last = url.trim_end_matches('/').rsplit('/').next().unwrap_or(url);
    let last = last.split(':').next_back().unwrap_or(last);
    last.trim_end_matches(".git").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_https_url() {
        assert_eq!(infer_dest_dir("https://github.com/foo/bar.git"), "bar");
        assert_eq!(infer_dest_dir("https://github.com/foo/bar"), "bar");
    }

    #[test]
    fn infer_ssh_url() {
        assert_eq!(infer_dest_dir("git@github.com:foo/bar.git"), "bar");
    }
}
