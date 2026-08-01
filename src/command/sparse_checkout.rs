//! `git sparse-checkout` — limit the working tree to a subset of paths.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Actions supported by `git sparse-checkout`.
#[derive(Debug, Clone)]
pub enum SparseCheckoutAction {
    /// `git sparse-checkout init [--[no-]cone] [--[no-]sparse-index]`: enable
    /// sparse checkout with a pattern set matching only the top-level files.
    Init {
        /// `--cone` / `--no-cone`.
        cone: Option<bool>,
        /// `--sparse-index` / `--no-sparse-index`.
        sparse_index: Option<bool>,
    },
    /// `git sparse-checkout set [<options>] <pattern>...`: replace the pattern
    /// set, enabling sparse checkout if it was off.
    Set {
        /// Patterns to write. At least one is required, so the first is taken
        /// by the constructor.
        patterns: Vec<String>,
        /// `--cone` / `--no-cone`.
        cone: Option<bool>,
        /// `--sparse-index` / `--no-sparse-index`.
        sparse_index: Option<bool>,
        /// `--skip-checks`.
        skip_checks: bool,
    },
    /// `git sparse-checkout add [--skip-checks] <pattern>...`: extend the
    /// pattern set.
    Add {
        /// Patterns to append. At least one is required, so the first is taken
        /// by the constructor.
        patterns: Vec<String>,
        /// `--skip-checks`.
        skip_checks: bool,
    },
    /// `git sparse-checkout list`: print the current pattern set.
    List,
    /// `git sparse-checkout disable`: restore a full checkout and clear
    /// `core.sparseCheckout`.
    Disable,
}

/// Builder for `git sparse-checkout`.
///
/// Sparse checkout keeps the whole repository history but populates only the
/// paths matching a pattern set. In the default cone mode a pattern is a
/// directory prefix; with `--no-cone` it is a full gitignore-style pattern,
/// which is why [`list`](Self::list) after a `--no-cone`
/// [`init`](Self::init) reports `/*` and `!/*/` rather than a path.
///
/// The five actions use the action-enum dispatch pattern: an option that does
/// not apply to the selected action is ignored rather than emitted.
///
/// This typed builder passes patterns as arguments. Command builders that
/// select `--stdin` can supply exact input with [`GitCommand::stdin_bytes`].
///
/// Output is left as a [`CommandOutput`]: `list` writes one pattern per line
/// and the other actions write nothing on stdout.
#[derive(Debug, Clone)]
pub struct SparseCheckoutCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action to perform.
    pub action: SparseCheckoutAction,
}

impl SparseCheckoutCommand {
    /// `sparse-checkout init`.
    pub fn init() -> Self {
        Self::with_action(SparseCheckoutAction::Init {
            cone: None,
            sparse_index: None,
        })
    }

    /// `sparse-checkout set <pattern>`.
    ///
    /// git requires at least one pattern, so the constructor takes it; add
    /// more with [`pattern`](Self::pattern).
    pub fn set(pattern: impl Into<String>) -> Self {
        Self::with_action(SparseCheckoutAction::Set {
            patterns: vec![pattern.into()],
            cone: None,
            sparse_index: None,
            skip_checks: false,
        })
    }

    /// `sparse-checkout add <pattern>`.
    ///
    /// git requires at least one pattern, so the constructor takes it; add
    /// more with [`pattern`](Self::pattern).
    pub fn add(pattern: impl Into<String>) -> Self {
        Self::with_action(SparseCheckoutAction::Add {
            patterns: vec![pattern.into()],
            skip_checks: false,
        })
    }

    /// `sparse-checkout list`.
    pub fn list() -> Self {
        Self::with_action(SparseCheckoutAction::List)
    }

    /// `sparse-checkout disable`.
    pub fn disable() -> Self {
        Self::with_action(SparseCheckoutAction::Disable)
    }

    /// Add another pattern (requires [`set`](Self::set) or [`add`](Self::add)).
    pub fn pattern(&mut self, pattern: impl Into<String>) -> &mut Self {
        match &mut self.action {
            SparseCheckoutAction::Set { patterns, .. }
            | SparseCheckoutAction::Add { patterns, .. } => patterns.push(pattern.into()),
            _ => {}
        }
        self
    }

    /// `--cone` (requires [`init`](Self::init) or [`set`](Self::set)).
    pub fn cone(&mut self) -> &mut Self {
        self.set_cone(true)
    }

    /// `--no-cone` (requires [`init`](Self::init) or [`set`](Self::set)).
    ///
    /// Cone mode is the default, so this is how a full-pattern checkout is
    /// requested.
    pub fn no_cone(&mut self) -> &mut Self {
        self.set_cone(false)
    }

    /// `--sparse-index` (requires [`init`](Self::init) or [`set`](Self::set)).
    pub fn sparse_index(&mut self) -> &mut Self {
        self.set_sparse_index(true)
    }

    /// `--no-sparse-index` (requires [`init`](Self::init) or
    /// [`set`](Self::set)).
    pub fn no_sparse_index(&mut self) -> &mut Self {
        self.set_sparse_index(false)
    }

    /// `--skip-checks` (requires [`set`](Self::set) or [`add`](Self::add)).
    ///
    /// Skips the sanity checks git runs on the given paths, which reject some
    /// legitimate patterns.
    pub fn skip_checks(&mut self) -> &mut Self {
        match &mut self.action {
            SparseCheckoutAction::Set { skip_checks, .. }
            | SparseCheckoutAction::Add { skip_checks, .. } => *skip_checks = true,
            _ => {}
        }
        self
    }

    fn set_cone(&mut self, value: bool) -> &mut Self {
        match &mut self.action {
            SparseCheckoutAction::Init { cone, .. } | SparseCheckoutAction::Set { cone, .. } => {
                *cone = Some(value);
            }
            _ => {}
        }
        self
    }

    fn set_sparse_index(&mut self, value: bool) -> &mut Self {
        match &mut self.action {
            SparseCheckoutAction::Init { sparse_index, .. }
            | SparseCheckoutAction::Set { sparse_index, .. } => *sparse_index = Some(value),
            _ => {}
        }
        self
    }

    fn with_action(action: SparseCheckoutAction) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action,
        }
    }
}

/// Push the `--[no-]<name>` form of an optional toggle.
fn push_toggle(args: &mut Vec<String>, name: &str, value: Option<bool>) {
    match value {
        Some(true) => args.push(format!("--{name}")),
        Some(false) => args.push(format!("--no-{name}")),
        None => {}
    }
}

#[async_trait]
impl GitCommand for SparseCheckoutCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["sparse-checkout".to_string()];
        match &self.action {
            SparseCheckoutAction::Init { cone, sparse_index } => {
                args.push("init".into());
                push_toggle(&mut args, "cone", *cone);
                push_toggle(&mut args, "sparse-index", *sparse_index);
            }
            SparseCheckoutAction::Set {
                patterns,
                cone,
                sparse_index,
                skip_checks,
            } => {
                args.push("set".into());
                push_toggle(&mut args, "cone", *cone);
                push_toggle(&mut args, "sparse-index", *sparse_index);
                if *skip_checks {
                    args.push("--skip-checks".into());
                }
                args.extend(patterns.iter().cloned());
            }
            SparseCheckoutAction::Add {
                patterns,
                skip_checks,
            } => {
                args.push("add".into());
                if *skip_checks {
                    args.push("--skip-checks".into());
                }
                args.extend(patterns.iter().cloned());
            }
            SparseCheckoutAction::List => args.push("list".into()),
            SparseCheckoutAction::Disable => args.push("disable".into()),
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
