//! `git bundle` — move objects and refs by archive file.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// How `git bundle` reports progress while writing a bundle.
///
/// `--quiet` and `--progress` set the same progress level inside git, so they
/// share one field here and the last call wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleProgress {
    /// `--quiet`: suppress the progress meter.
    Quiet,
    /// `--progress`: show the progress meter even when stderr is not a tty.
    Progress,
}

/// Actions supported by `git bundle`.
#[derive(Debug, Clone)]
pub enum BundleAction {
    /// `git bundle create [--quiet | --progress] [--version=<n>] <file> <rev-list-args>`.
    Create {
        /// Bundle file to write.
        file: PathBuf,
        /// Revision arguments naming what goes into the bundle.
        revs: Vec<String>,
        /// `--all`: include every ref.
        all: bool,
        /// Progress reporting.
        progress: Option<BundleProgress>,
        /// `--version=<n>`: bundle format version.
        version: Option<u32>,
    },
    /// `git bundle verify [--quiet] <file>`.
    Verify {
        /// Bundle file to check.
        file: PathBuf,
        /// `--quiet`.
        quiet: bool,
    },
    /// `git bundle list-heads <file> [<refname>...]`.
    ListHeads {
        /// Bundle file to read.
        file: PathBuf,
        /// Restrict the listing to these refs.
        refs: Vec<String>,
    },
    /// `git bundle unbundle [--progress] <file> [<refname>...]`.
    Unbundle {
        /// Bundle file to read.
        file: PathBuf,
        /// Restrict the unpacked refs to these.
        refs: Vec<String>,
        /// `--progress`.
        progress: bool,
    },
}

/// Builder for `git bundle`.
///
/// A bundle is a single file holding objects and refs, readable by `fetch` and
/// `clone` wherever a repository URL is accepted. The four actions use the
/// action-enum dispatch pattern: an option that does not apply to the selected
/// action is ignored rather than emitted.
///
/// `create` needs at least one revision, either [`all`](Self::all) or a
/// [`rev`](Self::rev); git refuses to write an empty bundle, and
/// [`execute`](GitCommand::execute) rejects that case before spawning.
///
/// Output is left as a [`CommandOutput`]: `verify` and `create` report on
/// stderr, while `list-heads` and `unbundle` write `<sha> <ref>` lines to
/// stdout.
#[derive(Debug, Clone)]
pub struct BundleCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Action to perform.
    pub action: BundleAction,
}

impl BundleCommand {
    /// `bundle create <file>`.
    pub fn create(file: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BundleAction::Create {
                file: file.into(),
                revs: Vec::new(),
                all: false,
                progress: None,
                version: None,
            },
        }
    }

    /// `bundle verify <file>`.
    pub fn verify(file: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BundleAction::Verify {
                file: file.into(),
                quiet: false,
            },
        }
    }

    /// `bundle list-heads <file>`.
    pub fn list_heads(file: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BundleAction::ListHeads {
                file: file.into(),
                refs: Vec::new(),
            },
        }
    }

    /// `bundle unbundle <file>`.
    pub fn unbundle(file: impl Into<PathBuf>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            action: BundleAction::Unbundle {
                file: file.into(),
                refs: Vec::new(),
                progress: false,
            },
        }
    }

    /// Add a revision argument (requires [`create`](Self::create)).
    ///
    /// Takes anything `git rev-list` accepts, so a ref name, a range such as
    /// `main~5..main`, or an option such as `--since=10.days`.
    pub fn rev(&mut self, rev: impl Into<String>) -> &mut Self {
        if let BundleAction::Create { revs, .. } = &mut self.action {
            revs.push(rev.into());
        }
        self
    }

    /// `--all` (requires [`create`](Self::create)).
    pub fn all(&mut self) -> &mut Self {
        if let BundleAction::Create { all, .. } = &mut self.action {
            *all = true;
        }
        self
    }

    /// `--version=<n>` (requires [`create`](Self::create)).
    pub fn version(&mut self, version: u32) -> &mut Self {
        if let BundleAction::Create { version: v, .. } = &mut self.action {
            *v = Some(version);
        }
        self
    }

    /// Restrict the refs acted on (requires [`list_heads`](Self::list_heads)
    /// or [`unbundle`](Self::unbundle)).
    pub fn ref_name(&mut self, name: impl Into<String>) -> &mut Self {
        match &mut self.action {
            BundleAction::ListHeads { refs, .. } | BundleAction::Unbundle { refs, .. } => {
                refs.push(name.into());
            }
            _ => {}
        }
        self
    }

    /// `--quiet` (requires [`create`](Self::create) or [`verify`](Self::verify)).
    pub fn quiet(&mut self) -> &mut Self {
        match &mut self.action {
            BundleAction::Create { progress, .. } => *progress = Some(BundleProgress::Quiet),
            BundleAction::Verify { quiet, .. } => *quiet = true,
            _ => {}
        }
        self
    }

    /// `--progress` (requires [`create`](Self::create) or
    /// [`unbundle`](Self::unbundle)).
    pub fn progress(&mut self) -> &mut Self {
        match &mut self.action {
            BundleAction::Create { progress, .. } => *progress = Some(BundleProgress::Progress),
            BundleAction::Unbundle { progress, .. } => *progress = true,
            _ => {}
        }
        self
    }
}

#[async_trait]
impl GitCommand for BundleCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["bundle".to_string()];
        match &self.action {
            BundleAction::Create {
                file,
                revs,
                all,
                progress,
                version,
            } => {
                args.push("create".into());
                match progress {
                    Some(BundleProgress::Quiet) => args.push("--quiet".into()),
                    Some(BundleProgress::Progress) => args.push("--progress".into()),
                    None => {}
                }
                if let Some(v) = version {
                    args.push(format!("--version={v}"));
                }
                args.push(file.display().to_string());
                if *all {
                    args.push("--all".into());
                }
                args.extend(revs.iter().cloned());
            }
            BundleAction::Verify { file, quiet } => {
                args.push("verify".into());
                if *quiet {
                    args.push("--quiet".into());
                }
                args.push(file.display().to_string());
            }
            BundleAction::ListHeads { file, refs } => {
                args.push("list-heads".into());
                args.push(file.display().to_string());
                args.extend(refs.iter().cloned());
            }
            BundleAction::Unbundle {
                file,
                refs,
                progress,
            } => {
                args.push("unbundle".into());
                if *progress {
                    args.push("--progress".into());
                }
                args.push(file.display().to_string());
                args.extend(refs.iter().cloned());
            }
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if let BundleAction::Create { revs, all, .. } = &self.action {
            if revs.is_empty() && !*all {
                return Err(Error::invalid_config(
                    "bundle create: needs at least one revision, call all() or rev()",
                ));
            }
        }
        self.execute_raw().await
    }
}
