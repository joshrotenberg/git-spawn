//! `git remote` — manage set of tracked repositories.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Actions supported by `git remote`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RemoteAction {
    /// List remotes (`git remote` or `git remote -v`).
    List {
        /// Verbose output.
        verbose: bool,
    },
    /// Add a remote: `git remote add <name> <url>`.
    Add {
        /// Remote name.
        name: String,
        /// Remote URL.
        url: String,
    },
    /// Remove a remote: `git remote remove <name>`.
    Remove(String),
    /// Rename a remote: `git remote rename <old> <new>`.
    Rename {
        /// Old name.
        from: String,
        /// New name.
        to: String,
    },
    /// Set URL: `git remote set-url <name> <url>`.
    SetUrl {
        /// Remote name.
        name: String,
        /// New URL.
        url: String,
    },
    /// Query URLs: `git remote get-url [--push] [--all] <name>`.
    ///
    /// Unlike reading `remote.<name>.url` from the configuration, `get-url`
    /// applies `insteadOf` and `pushInsteadOf` rewrites.
    GetUrl {
        /// Remote name.
        name: String,
        /// `--push`
        push: bool,
        /// `--all`
        all: bool,
    },
    /// Show remote: `git remote show <name>`.
    Show(String),
    /// Prune stale refs: `git remote prune <name>`.
    Prune(String),
}

/// Builder for `git remote`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RemoteCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action to perform.
    pub action: RemoteAction,
}

impl Default for RemoteCommand {
    fn default() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: RemoteAction::List { verbose: false },
        }
    }
}

impl RemoteCommand {
    /// List remotes.
    #[must_use]
    pub fn list() -> Self {
        Self {
            action: RemoteAction::List { verbose: false },
            ..Self::default()
        }
    }

    /// List remotes verbosely (`-v`).
    #[must_use]
    pub fn list_verbose() -> Self {
        Self {
            action: RemoteAction::List { verbose: true },
            ..Self::default()
        }
    }

    /// Add a remote.
    pub fn add(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            action: RemoteAction::Add {
                name: name.into(),
                url: url.into(),
            },
            ..Self::default()
        }
    }

    /// Remove a remote.
    pub fn remove(name: impl Into<String>) -> Self {
        Self {
            action: RemoteAction::Remove(name.into()),
            ..Self::default()
        }
    }

    /// Rename a remote.
    pub fn rename(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            action: RemoteAction::Rename {
                from: from.into(),
                to: to.into(),
            },
            ..Self::default()
        }
    }

    /// Change a remote's URL.
    pub fn set_url(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            action: RemoteAction::SetUrl {
                name: name.into(),
                url: url.into(),
            },
            ..Self::default()
        }
    }

    /// Query a remote's URL (`git remote get-url <name>`).
    ///
    /// Prints the fetch URL with `insteadOf` rewrites applied. Use
    /// [`push_url`](Self::push_url) for the push URL and [`all`](Self::all)
    /// for every configured URL, one per line.
    pub fn get_url(name: impl Into<String>) -> Self {
        Self {
            action: RemoteAction::GetUrl {
                name: name.into(),
                push: false,
                all: false,
            },
            ..Self::default()
        }
    }

    /// `--push` (requires [`get_url`](Self::get_url)).
    pub fn push_url(&mut self) -> &mut Self {
        if let RemoteAction::GetUrl { push, .. } = &mut self.action {
            *push = true;
        }
        self
    }

    /// `--all` (requires [`get_url`](Self::get_url)).
    pub fn all(&mut self) -> &mut Self {
        if let RemoteAction::GetUrl { all, .. } = &mut self.action {
            *all = true;
        }
        self
    }

    /// Show a remote.
    pub fn show(name: impl Into<String>) -> Self {
        Self {
            action: RemoteAction::Show(name.into()),
            ..Self::default()
        }
    }

    /// Prune a remote.
    pub fn prune(name: impl Into<String>) -> Self {
        Self {
            action: RemoteAction::Prune(name.into()),
            ..Self::default()
        }
    }
}

#[async_trait]
impl GitCommand for RemoteCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["remote".to_string()];
        match &self.action {
            RemoteAction::List { verbose } => {
                if *verbose {
                    args.push("-v".into());
                }
            }
            RemoteAction::Add { name, url } => {
                args.push("add".into());
                args.push(name.clone());
                args.push(url.clone());
            }
            RemoteAction::Remove(name) => {
                args.push("remove".into());
                args.push(name.clone());
            }
            RemoteAction::Rename { from, to } => {
                args.push("rename".into());
                args.push(from.clone());
                args.push(to.clone());
            }
            RemoteAction::SetUrl { name, url } => {
                args.push("set-url".into());
                args.push(name.clone());
                args.push(url.clone());
            }
            RemoteAction::GetUrl { name, push, all } => {
                args.push("get-url".into());
                if *push {
                    args.push("--push".into());
                }
                if *all {
                    args.push("--all".into());
                }
                args.push(name.clone());
            }
            RemoteAction::Show(name) => {
                args.push("show".into());
                args.push(name.clone());
            }
            RemoteAction::Prune(name) => {
                args.push("prune".into());
                args.push(name.clone());
            }
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
