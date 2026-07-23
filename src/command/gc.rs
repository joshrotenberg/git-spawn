//! `git gc` — clean up and optimize the local repository.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// How `git gc` should prune loose objects.
///
/// `--prune=<date>` and `--no-prune` are mutually exclusive, so they share one
/// field rather than two booleans that could contradict each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GcPrune {
    /// `--prune=<date>`: prune loose objects older than `<date>`. Use
    /// `"now"` to prune regardless of age.
    Before(String),
    /// `--no-prune`: keep every loose object.
    Never,
}

/// Builder for `git gc`.
///
/// Runs housekeeping: repacks objects, prunes unreachable ones, packs refs and
/// removes stale files. A bare `git gc` is a valid full run, so no field is
/// required.
///
/// Pruning is on by default with a two-week grace period. [`prune`] overrides
/// the cutoff and [`no_prune`] disables pruning entirely; calling both keeps
/// whichever was set last.
///
/// [`prune`]: GcCommand::prune
/// [`no_prune`]: GcCommand::no_prune
#[derive(Debug, Clone, Default)]
pub struct GcCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `--aggressive`: optimize more aggressively at the cost of more time.
    pub aggressive: bool,
    /// `--auto`: only run if housekeeping is due.
    pub auto: bool,
    /// The prune mode, if overriding git's default cutoff.
    pub prune: Option<GcPrune>,
}

impl GcCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Optimize more aggressively (`--aggressive`).
    pub fn aggressive(&mut self) -> &mut Self {
        self.aggressive = true;
        self
    }

    /// Only run when housekeeping is due (`--auto`).
    pub fn auto(&mut self) -> &mut Self {
        self.auto = true;
        self
    }

    /// Prune loose objects older than `date` (`--prune=<date>`). Pass `"now"`
    /// to prune regardless of age. Replaces any earlier [`no_prune`] call.
    ///
    /// [`no_prune`]: GcCommand::no_prune
    pub fn prune(&mut self, date: impl Into<String>) -> &mut Self {
        self.prune = Some(GcPrune::Before(date.into()));
        self
    }

    /// Keep every loose object (`--no-prune`). Replaces any earlier [`prune`]
    /// call.
    ///
    /// [`prune`]: GcCommand::prune
    pub fn no_prune(&mut self) -> &mut Self {
        self.prune = Some(GcPrune::Never);
        self
    }
}

#[async_trait]
impl GitCommand for GcCommand {
    /// Raw output. `git gc` writes its progress to `stderr`.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["gc".to_string()];
        if self.aggressive {
            args.push("--aggressive".into());
        }
        if self.auto {
            args.push("--auto".into());
        }
        match &self.prune {
            Some(GcPrune::Before(date)) => args.push(format!("--prune={date}")),
            Some(GcPrune::Never) => args.push("--no-prune".into()),
            None => {}
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
