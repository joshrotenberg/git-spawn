//! `git fsck` — verify the connectivity and validity of objects.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git fsck`.
///
/// Checks the object database for corruption and reports objects that nothing
/// references. A bare `git fsck` is a valid full check, so no field is
/// required.
///
/// Reports go to `stdout`, one object per line (`dangling blob <sha>`,
/// `unreachable commit <sha>`, and so on). Dangling objects alone are not an
/// error: git still exits 0.
///
/// Dangling reporting is on by default; [`no_dangling`] suppresses it and
/// [`dangling`] restores it, with the last call winning.
///
/// [`dangling`]: FsckCommand::dangling
/// [`no_dangling`]: FsckCommand::no_dangling
#[derive(Debug, Clone, Default)]
pub struct FsckCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `--full`: check every object in every pack and alternate object store.
    pub full: bool,
    /// `--unreachable`: report objects that no reference reaches.
    pub unreachable: bool,
    /// Whether to report dangling objects, if overriding git's default.
    pub dangling: Option<bool>,
}

impl FsckCommand {
    /// New command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check every packed object and alternate object store (`--full`).
    pub fn full(&mut self) -> &mut Self {
        self.full = true;
        self
    }

    /// Report objects no reference reaches (`--unreachable`).
    pub fn unreachable(&mut self) -> &mut Self {
        self.unreachable = true;
        self
    }

    /// Report dangling objects (`--dangling`). This is git's default; the
    /// option is here so a [`no_dangling`] call can be undone. Replaces any
    /// earlier [`no_dangling`] call.
    ///
    /// [`no_dangling`]: FsckCommand::no_dangling
    pub fn dangling(&mut self) -> &mut Self {
        self.dangling = Some(true);
        self
    }

    /// Do not report dangling objects (`--no-dangling`). Replaces any earlier
    /// [`dangling`] call.
    ///
    /// [`dangling`]: FsckCommand::dangling
    pub fn no_dangling(&mut self) -> &mut Self {
        self.dangling = Some(false);
        self
    }
}

#[async_trait]
impl GitCommand for FsckCommand {
    /// Raw output. `git fsck` writes its report to `stdout`.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["fsck".to_string()];
        if self.full {
            args.push("--full".into());
        }
        if self.unreachable {
            args.push("--unreachable".into());
        }
        match self.dangling {
            Some(true) => args.push("--dangling".into()),
            Some(false) => args.push("--no-dangling".into()),
            None => {}
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
