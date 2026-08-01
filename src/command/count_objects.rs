//! `git count-objects` — report object and disk usage statistics.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git count-objects`.
///
/// By default the command prints a one-line summary of the loose objects:
/// `<n> objects, <size> kilobytes`. [`verbose`](Self::verbose) switches to the
/// full statistic set, adding the packed-object, pack, prune-packable, and
/// garbage counts as `<key>: <value>` lines.
///
/// [`human_readable`](Self::human_readable) renders the sizes as text
/// (`20.00 KiB`, `0 bytes`) instead of bare kibibyte counts. It is a display
/// convenience: the typed parsers keep the text either way but can only report
/// a number for the bare form.
///
/// The command takes no arguments and cannot be built into an invalid shape, so
/// there is nothing to validate before spawning.
///
/// Output is left as a [`CommandOutput`];
/// [`parse_count_objects`](Self::parse_count_objects) turns `-v` output into a
/// typed [`CountObjects`](crate::parse::CountObjects), and
/// [`parse_count_objects_terse`](Self::parse_count_objects_terse) reads the
/// default one-line form.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct CountObjectsCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `-v`: report every statistic, one `<key>: <value>` line each.
    pub verbose: bool,
    /// `-H`: render sizes as text rather than bare kibibyte counts.
    pub human_readable: bool,
}

impl CountObjectsCommand {
    /// New command, reporting the default one-line summary.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Report every statistic rather than the loose-object summary.
    pub fn verbose(&mut self) -> &mut Self {
        self.verbose = true;
        self
    }

    /// Render sizes as text (`20.00 KiB`) rather than kibibyte counts.
    pub fn human_readable(&mut self) -> &mut Self {
        self.human_readable = true;
        self
    }

    /// Parse a completed [`verbose`](Self::verbose) run's output into typed
    /// statistics.
    ///
    /// Returns [`None`] for output that is missing a statistic, which includes
    /// the default one-line form; use
    /// [`parse_count_objects_terse`](Self::parse_count_objects_terse) for that.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_count_objects(
        &self,
        output: &CommandOutput,
    ) -> Option<crate::parse::CountObjects> {
        crate::parse::parse_count_objects(&output.stdout_str())
    }

    /// Parse a completed default-form run's output into the loose-object count
    /// and the space those objects occupy.
    ///
    /// Returns [`None`] for output of another shape, which includes
    /// [`verbose`](Self::verbose) output.
    #[cfg(feature = "parse")]
    #[must_use]
    pub fn parse_count_objects_terse(
        &self,
        output: &CommandOutput,
    ) -> Option<(u64, crate::parse::ObjectSize)> {
        crate::parse::parse_count_objects_terse(&output.stdout_str())
    }
}

#[async_trait]
impl GitCommand for CountObjectsCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["count-objects".to_string()];
        if self.verbose {
            args.push("-v".into());
        }
        if self.human_readable {
            args.push("-H".into());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
