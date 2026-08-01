//! `git maintenance` — run repository maintenance tasks and manage registration.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// A task `git maintenance run` knows how to execute.
///
/// [`Other`](Self::Other) passes a name through verbatim, so a task added by a
/// newer git can be requested without waiting for a variant here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaintenanceTask {
    /// `commit-graph`: write and verify the commit-graph file.
    CommitGraph,
    /// `prefetch`: fetch remote objects without updating remote-tracking refs.
    Prefetch,
    /// `gc`: the full `git gc` run.
    Gc,
    /// `loose-objects`: pack a batch of loose objects.
    LooseObjects,
    /// `incremental-repack`: repack into a multi-pack-index.
    IncrementalRepack,
    /// `pack-refs`: pack loose refs into a single file.
    PackRefs,
    /// A task name passed through as given.
    Other(String),
}

impl MaintenanceTask {
    /// The name git expects after `--task=`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::CommitGraph => "commit-graph",
            Self::Prefetch => "prefetch",
            Self::Gc => "gc",
            Self::LooseObjects => "loose-objects",
            Self::IncrementalRepack => "incremental-repack",
            Self::PackRefs => "pack-refs",
            Self::Other(name) => name,
        }
    }
}

/// The frequency `--schedule=<frequency>` selects tasks by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceSchedule {
    /// `hourly`.
    Hourly,
    /// `daily`.
    Daily,
    /// `weekly`.
    Weekly,
}

impl MaintenanceSchedule {
    /// The name git expects after `--schedule=`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
        }
    }
}

/// How `run` decides which of the configured tasks are due.
///
/// git rejects `--auto` and `--schedule=<frequency>` together, so the two live
/// in one field and cannot contradict each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaintenanceTrigger {
    /// `--auto`: only run tasks whose auto-condition is met.
    Auto,
    /// `--schedule=<frequency>`: only run tasks due at this frequency.
    Schedule(MaintenanceSchedule),
}

/// Actions supported by `git maintenance`.
#[derive(Debug, Clone)]
pub enum MaintenanceAction {
    /// `git maintenance run [--auto | --schedule=<f>] [--quiet] [--task=<t>]...`.
    Run {
        /// Tasks to run, overriding the configured set.
        tasks: Vec<MaintenanceTask>,
        /// `--auto` or `--schedule=<frequency>`.
        trigger: Option<MaintenanceTrigger>,
        /// `--quiet`.
        quiet: bool,
    },
    /// `git maintenance register [--config-file <path>]`.
    Register {
        /// `--config-file <path>`.
        config_file: Option<PathBuf>,
    },
    /// `git maintenance unregister [--force] [--config-file <path>]`.
    Unregister {
        /// `--config-file <path>`.
        config_file: Option<PathBuf>,
        /// `--force`.
        force: bool,
    },
}

/// Builder for `git maintenance`.
///
/// Covers the three actions that operate on the repository at hand: `run`
/// executes maintenance tasks now, `register` adds the repository to the list
/// the background scheduler visits, and `unregister` removes it again. The
/// `start` and `stop` actions are not modeled: they install and remove a
/// scheduler job on the host (cron, systemd, launchd, or the Windows task
/// scheduler), which is outside what a repository command should reach.
///
/// `register` and `unregister` write `maintenance.repo` to the user's global
/// config by default. [`config_file`](Self::config_file) redirects that write
/// to a given file, which is what the tests use to leave the real global
/// config alone.
///
/// Output is left as a [`CommandOutput`]: `run` reports progress on stderr and
/// the registration actions print nothing on success.
#[derive(Debug, Clone)]
pub struct MaintenanceCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action.
    pub action: MaintenanceAction,
}

impl MaintenanceCommand {
    /// `maintenance run`.
    #[must_use]
    pub fn run() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: MaintenanceAction::Run {
                tasks: Vec::new(),
                trigger: None,
                quiet: false,
            },
        }
    }

    /// `maintenance register`.
    #[must_use]
    pub fn register() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: MaintenanceAction::Register { config_file: None },
        }
    }

    /// `maintenance unregister`.
    #[must_use]
    pub fn unregister() -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: MaintenanceAction::Unregister {
                config_file: None,
                force: false,
            },
        }
    }

    /// Add a `--task=<task>` (requires [`run`](Self::run)).
    ///
    /// Repeatable: each call appends one task, and the requested tasks replace
    /// the configured set rather than adding to it.
    pub fn task(&mut self, task: MaintenanceTask) -> &mut Self {
        if let MaintenanceAction::Run { tasks, .. } = &mut self.action {
            tasks.push(task);
        }
        self
    }

    /// Add a `--task=<task>` by name (requires [`run`](Self::run)).
    pub fn task_raw(&mut self, task: impl Into<String>) -> &mut Self {
        self.task(MaintenanceTask::Other(task.into()))
    }

    /// `--auto` (requires [`run`](Self::run)).
    ///
    /// Shares a field with [`schedule`](Self::schedule); the last call wins.
    pub fn auto(&mut self) -> &mut Self {
        if let MaintenanceAction::Run { trigger, .. } = &mut self.action {
            *trigger = Some(MaintenanceTrigger::Auto);
        }
        self
    }

    /// `--schedule=<frequency>` (requires [`run`](Self::run)).
    ///
    /// Shares a field with [`auto`](Self::auto); the last call wins.
    pub fn schedule(&mut self, schedule: MaintenanceSchedule) -> &mut Self {
        if let MaintenanceAction::Run { trigger, .. } = &mut self.action {
            *trigger = Some(MaintenanceTrigger::Schedule(schedule));
        }
        self
    }

    /// `--quiet` (requires [`run`](Self::run)).
    pub fn quiet(&mut self) -> &mut Self {
        if let MaintenanceAction::Run { quiet, .. } = &mut self.action {
            *quiet = true;
        }
        self
    }

    /// `--config-file <path>` (requires [`register`](Self::register) or
    /// [`unregister`](Self::unregister)).
    pub fn config_file(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        match &mut self.action {
            MaintenanceAction::Register { config_file }
            | MaintenanceAction::Unregister { config_file, .. } => {
                *config_file = Some(path.into());
            }
            MaintenanceAction::Run { .. } => {}
        }
        self
    }

    /// `--force` (requires [`unregister`](Self::unregister)).
    ///
    /// Without it, unregistering a repository that is not registered fails.
    pub fn force(&mut self) -> &mut Self {
        if let MaintenanceAction::Unregister { force, .. } = &mut self.action {
            *force = true;
        }
        self
    }
}

#[async_trait]
impl GitCommand for MaintenanceCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["maintenance".to_string()];
        match &self.action {
            MaintenanceAction::Run {
                tasks,
                trigger,
                quiet,
            } => {
                args.push("run".into());
                match trigger {
                    Some(MaintenanceTrigger::Auto) => args.push("--auto".into()),
                    Some(MaintenanceTrigger::Schedule(s)) => {
                        args.push(format!("--schedule={}", s.as_str()));
                    }
                    None => {}
                }
                if *quiet {
                    args.push("--quiet".into());
                }
                for task in tasks {
                    args.push(format!("--task={}", task.as_str()));
                }
            }
            MaintenanceAction::Register { config_file } => {
                args.push("register".into());
                if let Some(path) = config_file {
                    args.push("--config-file".into());
                    args.push(path.display().to_string());
                }
            }
            MaintenanceAction::Unregister { config_file, force } => {
                args.push("unregister".into());
                if *force {
                    args.push("--force".into());
                }
                if let Some(path) = config_file {
                    args.push("--config-file".into());
                    args.push(path.display().to_string());
                }
            }
        }
        args
    }

    fn build_command_os_args(&self) -> Vec<std::ffi::OsString> {
        let mut args: Vec<_> = self
            .build_command_args()
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect();
        let config_file = match &self.action {
            MaintenanceAction::Register { config_file }
            | MaintenanceAction::Unregister { config_file, .. } => config_file.as_ref(),
            MaintenanceAction::Run { .. } => None,
        };
        if let Some(path) = config_file {
            *args.last_mut().expect("maintenance config file argument") =
                path.as_os_str().to_owned();
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
