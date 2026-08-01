//! `git rerere` — reuse recorded conflict resolutions.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Actions supported by `git rerere`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RerereAction {
    /// `git rerere status`: list the paths with a recorded preimage.
    Status,
    /// `git rerere diff`: show the current state of a conflict against its
    /// recorded preimage.
    Diff,
    /// `git rerere gc`: prune old records from the resolution cache.
    Gc,
    /// `git rerere clear`: drop the resolutions recorded for the merge in
    /// progress.
    Clear,
    /// `git rerere forget <pathspec>...`: drop the recorded resolution for
    /// the named paths.
    Forget {
        /// Paths whose resolution is forgotten. At least one is required, so
        /// the first is taken by the constructor.
        pathspecs: Vec<String>,
    },
}

/// Builder for `git rerere`.
///
/// When `rerere.enabled` is set, git records how a conflict was resolved and
/// replays that resolution the next time the same conflict appears. This
/// command inspects and maintains that cache; the recording itself happens
/// inside `merge`, `rebase` and friends.
///
/// The five actions use the action-enum dispatch pattern: an option that does
/// not apply to the selected action is ignored rather than emitted.
///
/// With `rerere.enabled` unset every action is a silent no-op that still exits
/// zero, so a successful run does not by itself mean anything was recorded.
///
/// Output is left as a [`CommandOutput`]: `status` writes one path per line,
/// `diff` writes a unified diff, and the maintenance actions write nothing on
/// stdout.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RerereCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action to perform.
    pub action: RerereAction,
}

impl RerereCommand {
    /// `rerere status`.
    pub fn status() -> Self {
        Self::with_action(RerereAction::Status)
    }

    /// `rerere diff`.
    pub fn diff() -> Self {
        Self::with_action(RerereAction::Diff)
    }

    /// `rerere gc`.
    ///
    /// Pruning is driven by the `gc.rerereResolved` and `gc.rerereUnresolved`
    /// config ages, and entries still referenced by a merge in progress are
    /// kept, so a freshly recorded resolution normally survives.
    pub fn gc() -> Self {
        Self::with_action(RerereAction::Gc)
    }

    /// `rerere clear`.
    pub fn clear() -> Self {
        Self::with_action(RerereAction::Clear)
    }

    /// `rerere forget <pathspec>`.
    ///
    /// git requires at least one pathspec, so the constructor takes it; add
    /// more with [`pathspec`](Self::pathspec).
    pub fn forget(pathspec: impl Into<String>) -> Self {
        Self::with_action(RerereAction::Forget {
            pathspecs: vec![pathspec.into()],
        })
    }

    /// Add another pathspec (requires [`forget`](Self::forget)).
    pub fn pathspec(&mut self, pathspec: impl Into<String>) -> &mut Self {
        if let RerereAction::Forget { pathspecs } = &mut self.action {
            pathspecs.push(pathspec.into());
        }
        self
    }

    fn with_action(action: RerereAction) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action,
        }
    }
}

#[async_trait]
impl GitCommand for RerereCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["rerere".to_string()];
        match &self.action {
            RerereAction::Status => args.push("status".into()),
            RerereAction::Diff => args.push("diff".into()),
            RerereAction::Gc => args.push("gc".into()),
            RerereAction::Clear => args.push("clear".into()),
            RerereAction::Forget { pathspecs } => {
                args.push("forget".into());
                args.extend(pathspecs.iter().cloned());
            }
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
