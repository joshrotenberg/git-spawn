//! `git ls-tree` — list the contents of a tree object.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git ls-tree`.
#[derive(Debug, Clone, Default)]
pub struct LsTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Tree-ish to list.
    pub tree: Option<String>,
    /// `-r` recurse into subtrees.
    pub recurse: bool,
    /// `-t` show trees even with `-r`.
    pub show_trees: bool,
    /// `-d` show only trees.
    pub trees_only: bool,
    /// `-l` include object size for blobs.
    pub long: bool,
    /// `--name-only`.
    pub name_only: bool,
    /// `--full-tree`.
    pub full_tree: bool,
    /// Pathspecs.
    pub paths: Vec<String>,
}

impl LsTreeCommand {
    /// New command with the given tree-ish.
    pub fn new(tree: impl Into<String>) -> Self {
        Self {
            tree: Some(tree.into()),
            ..Self::default()
        }
    }

    /// `-r`.
    pub fn recurse(&mut self) -> &mut Self {
        self.recurse = true;
        self
    }

    /// `-t`.
    pub fn show_trees(&mut self) -> &mut Self {
        self.show_trees = true;
        self
    }

    /// `-d`.
    pub fn trees_only(&mut self) -> &mut Self {
        self.trees_only = true;
        self
    }

    /// `-l`.
    pub fn long(&mut self) -> &mut Self {
        self.long = true;
        self
    }

    /// `--name-only`.
    pub fn name_only(&mut self) -> &mut Self {
        self.name_only = true;
        self
    }

    /// `--full-tree`.
    pub fn full_tree(&mut self) -> &mut Self {
        self.full_tree = true;
        self
    }

    /// Filter by path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }
}

#[async_trait]
impl GitCommand for LsTreeCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["ls-tree".to_string()];
        if self.recurse {
            args.push("-r".into());
        }
        if self.show_trees {
            args.push("-t".into());
        }
        if self.trees_only {
            args.push("-d".into());
        }
        if self.long {
            args.push("-l".into());
        }
        if self.name_only {
            args.push("--name-only".into());
        }
        if self.full_tree {
            args.push("--full-tree".into());
        }
        if let Some(t) = &self.tree {
            args.push(t.clone());
        }
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        if self.tree.is_none() {
            return Err(Error::invalid_config(
                "ls-tree requires a tree-ish argument",
            ));
        }
        self.execute_raw().await
    }
}
