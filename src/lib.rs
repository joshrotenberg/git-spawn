//! # git-wrapper
//!
//! A Rust wrapper around the `git` CLI. Each git subcommand is a struct with a
//! builder-style API; calling [`.execute().await`](GitCommand::execute) spawns
//! `git` as a subprocess and returns typed output.
//!
//! Unlike libraries that link against libgit2, this crate shells out to the
//! `git` binary installed on the host. That choice has trade-offs:
//!
//! | Pro                                          | Con                                    |
//! |----------------------------------------------|----------------------------------------|
//! | Behavior matches the user's local git        | Requires `git` on `PATH` at runtime    |
//! | Supports every flag — escape hatches for all | Higher per-call overhead than libgit2  |
//! | Honors `core.*` config, hooks, credentials   | Output parsing is up to you (or us!)   |
//!
//! If you want a scripting-like experience with the same flags you'd type in a
//! shell, this crate is for you. For in-process object database manipulation,
//! reach for [`git2`](https://docs.rs/git2) instead.
//!
//! ## Quick start
//!
//! ```no_run
//! use git_wrapper::{GitCommand, Repository};
//!
//! # async fn example() -> git_wrapper::Result<()> {
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Stage everything and commit.
//! repo.add().all().execute().await?;
//! repo.commit().message("snapshot").execute().await?;
//!
//! // Push to origin/main.
//! repo.push().remote("origin").refspec("main").execute().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Core concepts
//!
//! ### The `GitCommand` trait
//!
//! Every command struct implements [`GitCommand`]. That trait provides:
//!
//! - [`execute()`](GitCommand::execute) — run the command and decode output
//! - [`arg()`](GitCommand::arg) / [`args()`](GitCommand::args) — append raw CLI
//!   arguments (the universal escape hatch)
//! - [`current_dir()`](GitCommand::current_dir) — choose the working directory
//! - [`env()`](GitCommand::env) — set environment variables
//! - [`with_timeout()`](GitCommand::with_timeout) — cap execution time
//!
//! ### `Repository`
//!
//! [`Repository`] is a cheap, cloneable handle to a working tree. Its accessor
//! methods return commands pre-scoped to that path, so you rarely need to set
//! `.current_dir()` explicitly:
//!
//! ```no_run
//! # async fn ex() -> git_wrapper::Result<()> {
//! # use git_wrapper::{GitCommand, Repository};
//! let repo = Repository::open("/path/to/repo")?;
//! let status = repo.status().format(
//!     git_wrapper::command::status::StatusFormat::PorcelainV2
//! ).execute().await?;
//! println!("{}", status.stdout);
//! # Ok(())
//! # }
//! ```
//!
//! ### Typed parsers (the `parse` module)
//!
//! By default commands return [`CommandOutput`] — raw stdout/stderr plus the
//! exit status. For common outputs the [`parse`] module provides structured
//! types behind the `parse` feature (on by default):
//!
//! ```no_run
//! # async fn ex() -> git_wrapper::Result<()> {
//! # use git_wrapper::{GitCommand, Repository};
//! use git_wrapper::command::status::StatusFormat;
//! use git_wrapper::parse::{parse_status, StatusKind};
//!
//! let repo = Repository::open("/path/to/repo")?;
//! let out = repo.status()
//!     .format(StatusFormat::PorcelainV1)
//!     .null_terminate()
//!     .execute()
//!     .await?;
//! for entry in parse_status(&out.stdout)? {
//!     if entry.index == StatusKind::Modified {
//!         println!("modified in index: {}", entry.path);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature flags
//!
//! | Flag    | Default | Purpose                                       |
//! |---------|:-------:|-----------------------------------------------|
//! | `parse` |   on    | Typed parsers for status/log/diff output      |
//! | `serde` |   off   | `Serialize`/`Deserialize` on parsed types     |
//!
//! ## Error handling
//!
//! All methods return [`Result<T>`](Result). The [`Error`] enum distinguishes
//! common failure modes — `git` missing from `PATH`, a non-zero exit, a
//! timeout, an invalid builder configuration, a path that isn't a git repo,
//! and so on. The [`Error::CommandFailed`] variant carries the captured
//! `stdout`, `stderr`, and exit code so you can present a good error message
//! or retry.
//!
//! ## Design principles
//!
//! - **One struct per subcommand** under [`command`]
//! - **Async-first** on [`tokio`]
//! - **Raw output by default**; typed parsing is opt-in via the `parse` module
//! - **Escape hatches everywhere** so the crate is useful for flags we haven't
//!   wrapped yet
//! - **No unsafe code**, no global state, no hidden config

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

pub mod command;
pub mod error;
#[cfg(feature = "parse")]
#[cfg_attr(docsrs, doc(cfg(feature = "parse")))]
pub mod parse;
pub mod repo;

pub use command::{
    CommandExecutor, CommandOutput, GitCommand, add::AddCommand, bisect::BisectCommand,
    branch::BranchCommand, cat_file::CatFileCommand, checkout::CheckoutCommand,
    cherry_pick::CherryPickCommand, clone::CloneCommand, commit::CommitCommand,
    config::ConfigCommand, diff::DiffCommand, fetch::FetchCommand, for_each_ref::ForEachRefCommand,
    grep::GrepCommand, hash_object::HashObjectCommand, init::InitCommand, log::LogCommand,
    ls_files::LsFilesCommand, ls_tree::LsTreeCommand, merge::MergeCommand, mv::MvCommand,
    pull::PullCommand, push::PushCommand, rebase::RebaseCommand, reflog::ReflogCommand,
    remote::RemoteCommand, reset::ResetCommand, restore::RestoreCommand,
    rev_parse::RevParseCommand, rm::RmCommand, show::ShowCommand, stash::StashCommand,
    status::StatusCommand, submodule::SubmoduleCommand, switch::SwitchCommand, tag::TagCommand,
    update_ref::UpdateRefCommand, worktree::WorktreeCommand,
};
pub use error::{Error, Result};
pub use repo::Repository;
