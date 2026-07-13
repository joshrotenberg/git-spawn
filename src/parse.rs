//! Typed parsers for common git outputs.
//!
//! Enabled by the `parse` feature (on by default). Each parser takes a raw
//! [`&str`](str) of git output (typically captured via
//! [`CommandOutput::stdout`](crate::CommandOutput)) and returns structured
//! entries. Parsers are deliberately permissive: unexpected fields are
//! preserved in raw form rather than erroring, so callers can handle them
//! downstream.
//!
//! # Modules
//!
//! - [`status`] — parse `git status --porcelain=v1 -z` output
//! - [`log`] — parse `git log` output using a fixed format token string
//! - [`diff`] — parse `git diff --name-status -z` output
//! - [`notes`] — parse `git notes list` output into `(note, object)` pairs
//! - [`commit`] — parse `git commit` output into a [`commit::CommitResult`]
//! - [`merge`] — parse `git merge` output into a [`merge::MergeResult`]
//! - [`pull`] — classify `git pull` output into a [`PullResult`]
//! - [`rebase`] — classify `git rebase` output into a [`RebaseResult`]
//! - [`reflog`] — parse `git reflog show` output using a fixed format token string

pub mod commit;
pub mod diff;
pub mod log;
pub mod merge;
pub mod notes;
pub mod pull;
pub mod rebase;
pub mod reflog;
pub mod status;

pub use commit::{CommitResult, parse_commit};
pub use diff::{DiffEntry, DiffKind, parse_diff_name_status};
pub use log::{CommitEntry, LOG_FORMAT, parse_log};
pub use merge::{MergeResult, parse_merge};
pub use notes::parse_notes_list;
pub use pull::{PullResult, parse_pull};
pub use rebase::{RebaseResult, parse_rebase};
pub use reflog::{REFLOG_FORMAT, ReflogEntry, parse_reflog};
pub use status::{StatusEntry, StatusKind, parse_status};
