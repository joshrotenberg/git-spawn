//! `git worktree` — manage multiple working trees attached to the same repository.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Actions supported by `git worktree`.
#[derive(Debug, Clone)]
pub enum WorktreeAction {
    /// `git worktree add [-b <branch>] [--detach] [--force] <path> [<commit-ish>]`.
    Add {
        /// Path for the new worktree.
        path: PathBuf,
        /// Commit-ish to check out.
        commit_ish: Option<String>,
        /// `-b <branch>` create a new branch.
        new_branch: Option<String>,
        /// `--detach`.
        detach: bool,
        /// `--force`.
        force: bool,
        /// `--track`.
        track: bool,
    },
    /// `git worktree list [--porcelain]`.
    List {
        /// Emit porcelain format.
        porcelain: bool,
    },
    /// `git worktree remove <path>`.
    Remove {
        /// Worktree path.
        path: PathBuf,
        /// `--force`.
        force: bool,
    },
    /// `git worktree prune`.
    Prune {
        /// `-v` verbose.
        verbose: bool,
        /// `--dry-run`.
        dry_run: bool,
    },
    /// `git worktree move <source> <destination>`.
    Move {
        /// Current worktree path.
        source: PathBuf,
        /// New path.
        destination: PathBuf,
    },
    /// `git worktree lock <path> [--reason <s>]`.
    Lock {
        /// Worktree path.
        path: PathBuf,
        /// Optional reason.
        reason: Option<String>,
    },
    /// `git worktree unlock <path>`.
    Unlock {
        /// Worktree path.
        path: PathBuf,
    },
}

/// Builder for `git worktree`.
#[derive(Debug, Clone)]
pub struct WorktreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: WorktreeAction,
}

impl WorktreeCommand {
    /// `worktree add`.
    pub fn add(path: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::Add {
                path: path.into(),
                commit_ish: None,
                new_branch: None,
                detach: false,
                force: false,
                track: false,
            },
        }
    }

    /// Check out `commit_ish` in the new worktree (requires [`add`](Self::add)).
    #[must_use]
    pub fn commit_ish(mut self, c: impl Into<String>) -> Self {
        if let WorktreeAction::Add { commit_ish, .. } = &mut self.action {
            *commit_ish = Some(c.into());
        }
        self
    }

    /// Create a new branch at the new worktree (requires [`add`](Self::add)).
    #[must_use]
    pub fn new_branch(mut self, b: impl Into<String>) -> Self {
        if let WorktreeAction::Add { new_branch, .. } = &mut self.action {
            *new_branch = Some(b.into());
        }
        self
    }

    /// `--detach` (requires [`add`](Self::add)).
    #[must_use]
    pub fn detach(mut self) -> Self {
        if let WorktreeAction::Add { detach, .. } = &mut self.action {
            *detach = true;
        }
        self
    }

    /// `--force` (requires [`add`](Self::add) or [`remove`](Self::remove)).
    #[must_use]
    pub fn force(mut self) -> Self {
        match &mut self.action {
            WorktreeAction::Add { force, .. } | WorktreeAction::Remove { force, .. } => {
                *force = true;
            }
            _ => {}
        }
        self
    }

    /// `worktree list`.
    #[must_use]
    pub fn list() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::List { porcelain: false },
        }
    }

    /// `worktree list --porcelain`.
    #[must_use]
    pub fn list_porcelain() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::List { porcelain: true },
        }
    }

    /// `worktree remove`.
    pub fn remove(path: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::Remove {
                path: path.into(),
                force: false,
            },
        }
    }

    /// `worktree prune`.
    #[must_use]
    pub fn prune() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::Prune {
                verbose: false,
                dry_run: false,
            },
        }
    }

    /// `worktree move`.
    pub fn move_tree(source: impl Into<PathBuf>, destination: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::Move {
                source: source.into(),
                destination: destination.into(),
            },
        }
    }

    /// `worktree lock`.
    pub fn lock(path: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::Lock {
                path: path.into(),
                reason: None,
            },
        }
    }

    /// Attach a `--reason` (requires [`lock`](Self::lock)).
    #[must_use]
    pub fn reason(mut self, r: impl Into<String>) -> Self {
        if let WorktreeAction::Lock { reason, .. } = &mut self.action {
            *reason = Some(r.into());
        }
        self
    }

    /// `worktree unlock`.
    pub fn unlock(path: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: WorktreeAction::Unlock { path: path.into() },
        }
    }
}

#[async_trait]
impl GitCommand for WorktreeCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["worktree".to_string()];
        match &self.action {
            WorktreeAction::Add {
                path,
                commit_ish,
                new_branch,
                detach,
                force,
                track,
            } => {
                args.push("add".into());
                if *force {
                    args.push("--force".into());
                }
                if *detach {
                    args.push("--detach".into());
                }
                if *track {
                    args.push("--track".into());
                }
                if let Some(b) = new_branch {
                    args.push("-b".into());
                    args.push(b.clone());
                }
                args.push(path.display().to_string());
                if let Some(c) = commit_ish {
                    args.push(c.clone());
                }
            }
            WorktreeAction::List { porcelain } => {
                args.push("list".into());
                if *porcelain {
                    args.push("--porcelain".into());
                }
            }
            WorktreeAction::Remove { path, force } => {
                args.push("remove".into());
                if *force {
                    args.push("--force".into());
                }
                args.push(path.display().to_string());
            }
            WorktreeAction::Prune { verbose, dry_run } => {
                args.push("prune".into());
                if *verbose {
                    args.push("-v".into());
                }
                if *dry_run {
                    args.push("--dry-run".into());
                }
            }
            WorktreeAction::Move {
                source,
                destination,
            } => {
                args.push("move".into());
                args.push(source.display().to_string());
                args.push(destination.display().to_string());
            }
            WorktreeAction::Lock { path, reason } => {
                args.push("lock".into());
                if let Some(r) = reason {
                    args.push("--reason".into());
                    args.push(r.clone());
                }
                args.push(path.display().to_string());
            }
            WorktreeAction::Unlock { path } => {
                args.push("unlock".into());
                args.push(path.display().to_string());
            }
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
