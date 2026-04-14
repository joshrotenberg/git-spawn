//! # git-wrapper
//!
//! A Rust wrapper around the `git` CLI. Each git subcommand is a struct with
//! a builder-style API; calling `.execute().await` spawns `git` and returns
//! typed output.
//!
//! ```no_run
//! use git_wrapper::{GitCommand, Repository};
//!
//! # async fn example() -> git_wrapper::Result<()> {
//! let _repo = Repository::open("/path/to/repo")?;
//! let version = git_wrapper::command::git_version().await?;
//! println!("{version}");
//! # Ok(())
//! # }
//! ```
//!
//! ## Design
//!
//! - **One struct per subcommand**, implementing the [`GitCommand`] trait.
//! - **Async-first** on `tokio` — commands return futures so callers can run
//!   them concurrently.
//! - **Raw output by default**; typed parsers (status, log, diff, blame) live
//!   behind the `parse` feature.
//! - **Escape hatches** on every command (`.arg()`, `.args()`, `.flag()`,
//!   `.option()`) for flags the typed API doesn't cover yet.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod command;
pub mod error;
pub mod repo;

pub use command::{
    CommandExecutor, CommandOutput, GitCommand, add::AddCommand, branch::BranchCommand,
    checkout::CheckoutCommand, clone::CloneCommand, commit::CommitCommand, diff::DiffCommand,
    fetch::FetchCommand, init::InitCommand, log::LogCommand, merge::MergeCommand, mv::MvCommand,
    pull::PullCommand, push::PushCommand, rebase::RebaseCommand, remote::RemoteCommand,
    reset::ResetCommand, restore::RestoreCommand, rm::RmCommand, show::ShowCommand,
    stash::StashCommand, status::StatusCommand, switch::SwitchCommand, tag::TagCommand,
};
pub use error::{Error, Result};
pub use repo::Repository;
