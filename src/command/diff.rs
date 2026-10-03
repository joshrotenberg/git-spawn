//! `git diff` — show changes between commits, trees, and the working tree.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Builder for `git diff`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DiffCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `--cached` / `--staged`.
    pub cached: bool,
    /// `--name-only`.
    pub name_only: bool,
    /// `--name-status`.
    pub name_status: bool,
    /// `--stat`.
    pub stat: bool,
    /// `--shortstat`.
    pub shortstat: bool,
    /// `--numstat`.
    pub numstat: bool,
    /// `--no-color`.
    pub no_color: bool,
    /// `--quiet`: suppress the diff and use exit status to report changes.
    pub quiet: bool,
    /// NUL-terminate entries (`-z`).
    pub null_terminate: bool,
    /// `--unified=N`.
    pub unified: Option<u32>,
    /// Revisions (e.g. `HEAD~1 HEAD`).
    pub revisions: Vec<String>,
    /// Pathspec filters.
    pub paths: Vec<String>,
}

impl DiffCommand {
    /// New `diff` command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Show staged changes.
    pub fn cached(&mut self) -> &mut Self {
        self.cached = true;
        self
    }

    /// `--name-only`.
    pub fn name_only(&mut self) -> &mut Self {
        self.name_only = true;
        self
    }

    /// `--name-status`.
    pub fn name_status(&mut self) -> &mut Self {
        self.name_status = true;
        self
    }

    /// `--stat`.
    pub fn stat(&mut self) -> &mut Self {
        self.stat = true;
        self
    }

    /// `--shortstat`.
    pub fn shortstat(&mut self) -> &mut Self {
        self.shortstat = true;
        self
    }

    /// `--numstat`.
    pub fn numstat(&mut self) -> &mut Self {
        self.numstat = true;
        self
    }

    /// Disable color.
    pub fn no_color(&mut self) -> &mut Self {
        self.no_color = true;
        self
    }

    /// Suppress output and enable Git's difference exit status.
    ///
    /// [`GitCommand::execute`] treats exit status 1 as a command failure.
    /// Use [`has_changes`](Self::has_changes) to interpret it as a difference.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }

    /// Return whether the selected working tree, index, revisions or paths
    /// differ. Executes with `--quiet`: exit 0 means unchanged, exit 1 means
    /// changed, and all other statuses or execution failures remain errors.
    ///
    /// Untracked files are outside `git diff` and are not counted. Like Git,
    /// this disables external diff helpers unless they opt into trusted exit
    /// codes. Other builder settings and executor policies are preserved.
    pub async fn has_changes(&self) -> Result<bool> {
        let mut command = self.clone();
        command.quiet = true;
        let output = command
            .executor
            .execute_command_os_allowing(command.build_command_os_args(), &[0, 1])
            .await?;
        Ok(output.exit_code == 1)
    }

    /// NUL-separate entries (`-z`). Required to safely parse
    /// [`--numstat`](Self::numstat) or [`--name-status`](Self::name_status)
    /// output via [`parse_diff_numstat`](crate::parse::parse_diff_numstat) or
    /// [`parse_diff_name_status`](crate::parse::parse_diff_name_status).
    pub fn null_terminate(&mut self) -> &mut Self {
        self.null_terminate = true;
        self
    }

    /// Context lines (`-U`).
    pub fn unified(&mut self, n: u32) -> &mut Self {
        self.unified = Some(n);
        self
    }

    /// Add a revision.
    pub fn revision(&mut self, r: impl Into<String>) -> &mut Self {
        self.revisions.push(r.into());
        self
    }

    /// Filter by path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }
}

#[async_trait]
impl GitCommand for DiffCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["diff".to_string()];
        if self.cached {
            args.push("--cached".into());
        }
        if self.name_only {
            args.push("--name-only".into());
        }
        if self.name_status {
            args.push("--name-status".into());
        }
        if self.stat {
            args.push("--stat".into());
        }
        if self.shortstat {
            args.push("--shortstat".into());
        }
        if self.numstat {
            args.push("--numstat".into());
        }
        if self.no_color {
            args.push("--no-color".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if self.null_terminate {
            args.push("-z".into());
        }
        if let Some(u) = self.unified {
            args.push(format!("--unified={u}"));
        }
        args.extend(self.revisions.iter().cloned());
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
