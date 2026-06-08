//! `git reflog` — manage and inspect reflog information.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Actions supported by `git reflog`.
#[derive(Debug, Clone)]
pub enum ReflogAction {
    /// `git reflog [show] [<ref>]`.
    Show {
        /// Ref to inspect (default `HEAD`).
        ref_name: Option<String>,
        /// `-n N` / `--max-count=N`.
        max_count: Option<u32>,
        /// Extra arbitrary format.
        format: Option<String>,
    },
    /// `git reflog expire [options] <refs>`.
    Expire {
        /// `--all`.
        all: bool,
        /// `--expire=<time>`.
        expire: Option<String>,
        /// `--expire-unreachable=<time>`.
        expire_unreachable: Option<String>,
        /// `--stale-fix`.
        stale_fix: bool,
        /// Refs to expire.
        refs: Vec<String>,
    },
    /// `git reflog delete <entry>…`.
    Delete {
        /// Entries to delete (e.g. `HEAD@{0}`).
        entries: Vec<String>,
        /// `--rewrite`.
        rewrite: bool,
    },
    /// `git reflog exists <ref>`.
    Exists {
        /// Ref to check.
        ref_name: String,
    },
}

/// Builder for `git reflog`.
#[derive(Debug, Clone)]
pub struct ReflogCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: ReflogAction,
}

impl ReflogCommand {
    /// `reflog` / `reflog show`.
    #[must_use]
    pub fn show() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ReflogAction::Show {
                ref_name: None,
                max_count: None,
                format: None,
            },
        }
    }

    /// Set the ref (for `show`).
    pub fn ref_name(&mut self, r: impl Into<String>) -> &mut Self {
        if let ReflogAction::Show { ref_name, .. } = &mut self.action {
            *ref_name = Some(r.into());
        }
        self
    }

    /// `-n` / `--max-count` (for `show`).
    pub fn max_count(&mut self, n: u32) -> &mut Self {
        if let ReflogAction::Show { max_count, .. } = &mut self.action {
            *max_count = Some(n);
        }
        self
    }

    /// Set `--format` (for `show`).
    pub fn format(&mut self, f: impl Into<String>) -> &mut Self {
        if let ReflogAction::Show { format, .. } = &mut self.action {
            *format = Some(f.into());
        }
        self
    }

    /// `reflog expire`.
    #[must_use]
    pub fn expire() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ReflogAction::Expire {
                all: false,
                expire: None,
                expire_unreachable: None,
                stale_fix: false,
                refs: vec![],
            },
        }
    }

    /// `reflog delete`.
    #[must_use]
    pub fn delete(entries: Vec<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ReflogAction::Delete {
                entries,
                rewrite: false,
            },
        }
    }

    /// `reflog exists <ref>`.
    pub fn exists(r: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: ReflogAction::Exists { ref_name: r.into() },
        }
    }
}

#[async_trait]
impl GitCommand for ReflogCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["reflog".to_string()];
        match &self.action {
            ReflogAction::Show {
                ref_name,
                max_count,
                format,
            } => {
                args.push("show".into());
                if let Some(n) = max_count {
                    args.push(format!("-n{n}"));
                }
                if let Some(f) = format {
                    args.push(format!("--format={f}"));
                }
                if let Some(r) = ref_name {
                    args.push(r.clone());
                }
            }
            ReflogAction::Expire {
                all,
                expire,
                expire_unreachable,
                stale_fix,
                refs,
            } => {
                args.push("expire".into());
                if *all {
                    args.push("--all".into());
                }
                if *stale_fix {
                    args.push("--stale-fix".into());
                }
                if let Some(e) = expire {
                    args.push(format!("--expire={e}"));
                }
                if let Some(e) = expire_unreachable {
                    args.push(format!("--expire-unreachable={e}"));
                }
                args.extend(refs.iter().cloned());
            }
            ReflogAction::Delete { entries, rewrite } => {
                args.push("delete".into());
                if *rewrite {
                    args.push("--rewrite".into());
                }
                args.extend(entries.iter().cloned());
            }
            ReflogAction::Exists { ref_name } => {
                args.push("exists".into());
                args.push(ref_name.clone());
            }
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
