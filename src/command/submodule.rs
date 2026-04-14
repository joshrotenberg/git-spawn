//! `git submodule` — initialize, update, or inspect submodules.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Actions supported by `git submodule`.
#[derive(Debug, Clone)]
pub enum SubmoduleAction {
    /// `git submodule add <url> [<path>]`.
    Add {
        /// Submodule URL.
        url: String,
        /// Optional path where the submodule is placed.
        path: Option<PathBuf>,
        /// `-b <branch>`.
        branch: Option<String>,
        /// `--force`.
        force: bool,
    },
    /// `git submodule init [<paths>…]`.
    Init {
        /// Restrict to these paths.
        paths: Vec<PathBuf>,
    },
    /// `git submodule update [<options>] [<paths>…]`.
    Update {
        /// `--init`.
        init: bool,
        /// `--recursive`.
        recursive: bool,
        /// `--remote`.
        remote: bool,
        /// `--force`.
        force: bool,
        /// Restrict to these paths.
        paths: Vec<PathBuf>,
    },
    /// `git submodule status [<paths>…]`.
    Status {
        /// `--cached`.
        cached: bool,
        /// `--recursive`.
        recursive: bool,
        /// Restrict to these paths.
        paths: Vec<PathBuf>,
    },
    /// `git submodule foreach <cmd>`.
    Foreach {
        /// Command to run inside each submodule.
        command: String,
        /// `--recursive`.
        recursive: bool,
    },
    /// `git submodule deinit <paths>`.
    Deinit {
        /// `--force`.
        force: bool,
        /// `--all`.
        all: bool,
        /// Paths to deinit.
        paths: Vec<PathBuf>,
    },
    /// `git submodule sync [<paths>…]`.
    Sync {
        /// `--recursive`.
        recursive: bool,
        /// Restrict to these paths.
        paths: Vec<PathBuf>,
    },
}

/// Builder for `git submodule`.
#[derive(Debug, Clone)]
pub struct SubmoduleCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: SubmoduleAction,
}

impl SubmoduleCommand {
    /// `submodule add`.
    pub fn add(url: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SubmoduleAction::Add {
                url: url.into(),
                path: None,
                branch: None,
                force: false,
            },
        }
    }

    /// Set the submodule path (for `add`).
    #[must_use]
    pub fn path(mut self, p: impl Into<PathBuf>) -> Self {
        if let SubmoduleAction::Add { path, .. } = &mut self.action {
            *path = Some(p.into());
        }
        self
    }

    /// Set `-b` branch (for `add`).
    #[must_use]
    pub fn branch(mut self, b: impl Into<String>) -> Self {
        if let SubmoduleAction::Add { branch, .. } = &mut self.action {
            *branch = Some(b.into());
        }
        self
    }

    /// Set `--force` (for `add` / `update` / `deinit`).
    #[must_use]
    pub fn force(mut self) -> Self {
        match &mut self.action {
            SubmoduleAction::Add { force, .. }
            | SubmoduleAction::Update { force, .. }
            | SubmoduleAction::Deinit { force, .. } => {
                *force = true;
            }
            _ => {}
        }
        self
    }

    /// `submodule init`.
    #[must_use]
    pub fn init() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SubmoduleAction::Init { paths: vec![] },
        }
    }

    /// `submodule update`.
    #[must_use]
    pub fn update() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SubmoduleAction::Update {
                init: false,
                recursive: false,
                remote: false,
                force: false,
                paths: vec![],
            },
        }
    }

    /// `--init` (for `update`).
    #[must_use]
    pub fn with_init(mut self) -> Self {
        if let SubmoduleAction::Update { init, .. } = &mut self.action {
            *init = true;
        }
        self
    }

    /// `--recursive` (for `update` / `status` / `foreach` / `sync`).
    #[must_use]
    pub fn recursive(mut self) -> Self {
        match &mut self.action {
            SubmoduleAction::Update { recursive, .. }
            | SubmoduleAction::Status { recursive, .. }
            | SubmoduleAction::Foreach { recursive, .. }
            | SubmoduleAction::Sync { recursive, .. } => {
                *recursive = true;
            }
            _ => {}
        }
        self
    }

    /// `--remote` (for `update`).
    #[must_use]
    pub fn remote(mut self) -> Self {
        if let SubmoduleAction::Update { remote, .. } = &mut self.action {
            *remote = true;
        }
        self
    }

    /// Restrict to a given path.
    #[must_use]
    pub fn restrict_path(mut self, p: impl Into<PathBuf>) -> Self {
        let p = p.into();
        match &mut self.action {
            SubmoduleAction::Init { paths }
            | SubmoduleAction::Update { paths, .. }
            | SubmoduleAction::Status { paths, .. }
            | SubmoduleAction::Deinit { paths, .. }
            | SubmoduleAction::Sync { paths, .. } => {
                paths.push(p);
            }
            _ => {}
        }
        self
    }

    /// `submodule status`.
    #[must_use]
    pub fn status() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SubmoduleAction::Status {
                cached: false,
                recursive: false,
                paths: vec![],
            },
        }
    }

    /// `--cached` (for `status`).
    #[must_use]
    pub fn cached(mut self) -> Self {
        if let SubmoduleAction::Status { cached, .. } = &mut self.action {
            *cached = true;
        }
        self
    }

    /// `submodule foreach`.
    pub fn foreach(command: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SubmoduleAction::Foreach {
                command: command.into(),
                recursive: false,
            },
        }
    }

    /// `submodule deinit`.
    #[must_use]
    pub fn deinit() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SubmoduleAction::Deinit {
                force: false,
                all: false,
                paths: vec![],
            },
        }
    }

    /// `--all` (for `deinit`).
    #[must_use]
    pub fn all(mut self) -> Self {
        if let SubmoduleAction::Deinit { all, .. } = &mut self.action {
            *all = true;
        }
        self
    }

    /// `submodule sync`.
    #[must_use]
    pub fn sync() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: SubmoduleAction::Sync {
                recursive: false,
                paths: vec![],
            },
        }
    }
}

#[async_trait]
impl GitCommand for SubmoduleCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["submodule".to_string()];
        match &self.action {
            SubmoduleAction::Add {
                url,
                path,
                branch,
                force,
            } => {
                args.push("add".into());
                if *force {
                    args.push("--force".into());
                }
                if let Some(b) = branch {
                    args.push("-b".into());
                    args.push(b.clone());
                }
                args.push(url.clone());
                if let Some(p) = path {
                    args.push(p.display().to_string());
                }
            }
            SubmoduleAction::Init { paths } => {
                args.push("init".into());
                if !paths.is_empty() {
                    args.push("--".into());
                    args.extend(paths.iter().map(|p| p.display().to_string()));
                }
            }
            SubmoduleAction::Update {
                init,
                recursive,
                remote,
                force,
                paths,
            } => {
                args.push("update".into());
                if *init {
                    args.push("--init".into());
                }
                if *recursive {
                    args.push("--recursive".into());
                }
                if *remote {
                    args.push("--remote".into());
                }
                if *force {
                    args.push("--force".into());
                }
                if !paths.is_empty() {
                    args.push("--".into());
                    args.extend(paths.iter().map(|p| p.display().to_string()));
                }
            }
            SubmoduleAction::Status {
                cached,
                recursive,
                paths,
            } => {
                args.push("status".into());
                if *cached {
                    args.push("--cached".into());
                }
                if *recursive {
                    args.push("--recursive".into());
                }
                if !paths.is_empty() {
                    args.push("--".into());
                    args.extend(paths.iter().map(|p| p.display().to_string()));
                }
            }
            SubmoduleAction::Foreach { command, recursive } => {
                args.push("foreach".into());
                if *recursive {
                    args.push("--recursive".into());
                }
                args.push(command.clone());
            }
            SubmoduleAction::Deinit { force, all, paths } => {
                args.push("deinit".into());
                if *force {
                    args.push("--force".into());
                }
                if *all {
                    args.push("--all".into());
                }
                if !paths.is_empty() {
                    args.push("--".into());
                    args.extend(paths.iter().map(|p| p.display().to_string()));
                }
            }
            SubmoduleAction::Sync { recursive, paths } => {
                args.push("sync".into());
                if *recursive {
                    args.push("--recursive".into());
                }
                if !paths.is_empty() {
                    args.push("--".into());
                    args.extend(paths.iter().map(|p| p.display().to_string()));
                }
            }
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
