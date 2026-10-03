//! `git push` — update remote refs along with associated objects.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;

/// Which remote state `git push` must find before it overwrites a ref.
///
/// The bare and valued spellings of `--force-with-lease` are mutually
/// exclusive, so they share one field rather than several that could
/// contradict each other.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PushForceWithLease {
    /// `--force-with-lease`: lease every updated ref against whatever the
    /// local remote-tracking ref currently holds.
    Implicit,
    /// `--force-with-lease=<refname>`: lease `refname` against its local
    /// remote-tracking ref.
    Ref(String),
    /// `--force-with-lease=<refname>:<expect>`: lease `refname` against an
    /// explicit expected object. An empty `expect` means the ref is expected
    /// not to exist.
    RefExpecting {
        /// The remote ref being leased.
        refname: String,
        /// The object the remote ref is expected to point at.
        expect: String,
    },
}

/// How `git push` should handle submodules (`--recurse-submodules=<mode>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PushRecurseSubmodules {
    /// `check`: fail if any submodule commit to be pushed is missing on its
    /// remote.
    Check,
    /// `on-demand`: push changed submodules first.
    OnDemand,
    /// `only`: push the submodules, not the superproject.
    Only,
    /// `no`: ignore submodules.
    No,
}

impl PushRecurseSubmodules {
    /// The value git spells for this mode.
    fn as_str(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::OnDemand => "on-demand",
            Self::Only => "only",
            Self::No => "no",
        }
    }
}

/// Builder for `git push`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct PushCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Remote name.
    pub remote: Option<String>,
    /// Refspecs.
    pub refspecs: Vec<String>,
    /// `--all`.
    pub all: bool,
    /// `--tags`.
    pub tags: bool,
    /// `--follow-tags` when `Some(true)`, `--no-follow-tags` when
    /// `Some(false)`.
    pub follow_tags: Option<bool>,
    /// `--force` / `-f`.
    pub force: bool,
    /// The lease mode, if pushing under `--force-with-lease`.
    pub force_with_lease: Option<PushForceWithLease>,
    /// `--delete`.
    pub delete: bool,
    /// `--set-upstream` / `-u`.
    pub set_upstream: bool,
    /// `--dry-run` / `-n`.
    pub dry_run: bool,
    /// `--atomic`.
    pub atomic: bool,
    /// `--quiet`.
    pub quiet: bool,
    /// `--no-verify`.
    pub no_verify: bool,
    /// `--recurse-submodules=<mode>`.
    pub recurse_submodules: Option<PushRecurseSubmodules>,
}

impl PushCommand {
    /// New `push`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Remote.
    pub fn remote(&mut self, r: impl Into<String>) -> &mut Self {
        self.remote = Some(r.into());
        self
    }
    /// Add a refspec.
    pub fn refspec(&mut self, r: impl Into<String>) -> &mut Self {
        self.refspecs.push(r.into());
        self
    }
    /// `--all`.
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }
    /// `--tags`.
    pub fn tags(&mut self) -> &mut Self {
        self.tags = true;
        self
    }
    /// `--follow-tags`. Replaces any earlier [`no_follow_tags`] call.
    ///
    /// [`no_follow_tags`]: PushCommand::no_follow_tags
    pub fn follow_tags(&mut self) -> &mut Self {
        self.follow_tags = Some(true);
        self
    }
    /// `--no-follow-tags`. Replaces any earlier [`follow_tags`] call.
    ///
    /// [`follow_tags`]: PushCommand::follow_tags
    pub fn no_follow_tags(&mut self) -> &mut Self {
        self.follow_tags = Some(false);
        self
    }
    /// `--force`.
    pub fn force(&mut self) -> &mut Self {
        self.force = true;
        self
    }
    /// `--force-with-lease`, leasing against the local remote-tracking refs.
    ///
    /// Replaces any earlier [`force_with_lease_for`] call.
    ///
    /// [`force_with_lease_for`]: PushCommand::force_with_lease_for
    pub fn force_with_lease(&mut self) -> &mut Self {
        self.force_with_lease = Some(PushForceWithLease::Implicit);
        self
    }
    /// `--force-with-lease=<refname>` or `--force-with-lease=<refname>:<expect>`,
    /// leasing `refname` against an explicit expected object.
    ///
    /// Passing `None` for `expect` leases against the local remote-tracking
    /// ref, as the single-argument form does; passing `Some("")` expects the
    /// ref not to exist. Unlike the bare form, an explicit `expect` is not
    /// moved by a concurrent fetch, so it fences the push deterministically.
    ///
    /// Replaces any earlier [`force_with_lease`] call.
    ///
    /// [`force_with_lease`]: PushCommand::force_with_lease
    pub fn force_with_lease_for(
        &mut self,
        refname: impl Into<String>,
        expect: Option<String>,
    ) -> &mut Self {
        let refname = refname.into();
        self.force_with_lease = Some(match expect {
            Some(expect) => PushForceWithLease::RefExpecting { refname, expect },
            None => PushForceWithLease::Ref(refname),
        });
        self
    }
    /// `--delete`.
    pub fn delete(&mut self) -> &mut Self {
        self.delete = true;
        self
    }
    /// `-u` / `--set-upstream`.
    pub fn set_upstream(&mut self) -> &mut Self {
        self.set_upstream = true;
        self
    }
    /// `--dry-run`.
    pub fn dry_run(&mut self) -> &mut Self {
        self.dry_run = true;
        self
    }
    /// `--atomic`.
    pub fn atomic(&mut self) -> &mut Self {
        self.atomic = true;
        self
    }
    /// `--quiet`.
    pub fn quiet(&mut self) -> &mut Self {
        self.quiet = true;
        self
    }
    /// `--no-verify`: skip the `pre-push` hook.
    pub fn no_verify(&mut self) -> &mut Self {
        self.no_verify = true;
        self
    }
    /// `--recurse-submodules=<mode>`.
    pub fn recurse_submodules(&mut self, mode: PushRecurseSubmodules) -> &mut Self {
        self.recurse_submodules = Some(mode);
        self
    }
}

#[async_trait]
impl GitCommand for PushCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["push".to_string()];
        if self.all {
            args.push("--all".into());
        }
        if self.tags {
            args.push("--tags".into());
        }
        match self.follow_tags {
            Some(true) => args.push("--follow-tags".into()),
            Some(false) => args.push("--no-follow-tags".into()),
            None => {}
        }
        if self.force {
            args.push("--force".into());
        }
        match &self.force_with_lease {
            Some(PushForceWithLease::Implicit) => args.push("--force-with-lease".into()),
            Some(PushForceWithLease::Ref(refname)) => {
                args.push(format!("--force-with-lease={refname}"));
            }
            Some(PushForceWithLease::RefExpecting { refname, expect }) => {
                args.push(format!("--force-with-lease={refname}:{expect}"));
            }
            None => {}
        }
        if self.delete {
            args.push("--delete".into());
        }
        if self.set_upstream {
            args.push("--set-upstream".into());
        }
        if self.dry_run {
            args.push("--dry-run".into());
        }
        if self.atomic {
            args.push("--atomic".into());
        }
        if self.quiet {
            args.push("--quiet".into());
        }
        if self.no_verify {
            args.push("--no-verify".into());
        }
        if let Some(mode) = self.recurse_submodules {
            args.push(format!("--recurse-submodules={}", mode.as_str()));
        }
        if let Some(r) = &self.remote {
            args.push(r.clone());
        }
        args.extend(self.refspecs.iter().cloned());
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}
