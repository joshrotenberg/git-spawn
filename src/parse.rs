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
//! - [`ls_tree`] — parse `git ls-tree` output into [`ls_tree::TreeEntry`] entries
//! - [`reflog`] — parse `git reflog show` output using a fixed format token string
//! - [`bisect`] — classify `git bisect` output into a [`bisect::BisectResult`]

pub mod bisect;
pub mod commit;
pub mod diff;
pub mod log;
pub mod ls_tree;
pub mod merge;
pub mod notes;
pub mod pull;
pub mod reflog;
pub mod status;

pub use bisect::{BisectResult, BisectStatus, parse_bisect};
pub use commit::{CommitResult, parse_commit};
pub use diff::{DiffEntry, DiffKind, parse_diff_name_status};
pub use log::{CommitEntry, LOG_FORMAT, parse_log};
pub use ls_tree::{TreeEntry, TreeObjectType, parse_ls_tree, parse_ls_tree_name_only};
pub use merge::{MergeResult, parse_merge};
pub use notes::parse_notes_list;
pub use pull::{PullResult, parse_pull};
pub use reflog::{REFLOG_FORMAT, ReflogEntry, parse_reflog};
pub use status::{StatusEntry, StatusKind, parse_status};
