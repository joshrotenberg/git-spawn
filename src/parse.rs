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
//! - [`cherry_pick`] — classify `git cherry-pick` output into a [`cherry_pick::CherryPickResult`]
//! - [`commit`] — parse `git commit` output into a [`commit::CommitResult`]
//! - [`diff`] — parse `git diff --name-status -z` output
//! - [`log`] — parse `git log` output using a fixed format token string
//! - [`ls_tree`] — parse `git ls-tree` output into [`ls_tree::TreeEntry`] entries
//! - [`merge`] — parse `git merge` output into a [`merge::MergeResult`]
//! - [`notes`] — parse `git notes list` output into `(note, object)` pairs
//! - [`pull`] — classify `git pull` output into a [`PullResult`]
//! - [`rebase`] — classify `git rebase` output into a [`RebaseResult`]
//! - [`reflog`] — parse `git reflog show` output using a fixed format token string
//! - [`show`] — parse `git show` output into a [`show::ShowResult`]
//! - [`status`] — parse `git status --porcelain=v1 -z` output, with
//!   [`status::parse_full_status`] additionally covering the `-b` branch header
//! - [`submodule`] — parse `git submodule status` output into [`submodule::SubmoduleEntry`] entries

pub mod cherry_pick;
pub mod commit;
pub mod diff;
pub mod log;
pub mod ls_tree;
pub mod merge;
pub mod notes;
pub mod pull;
pub mod rebase;
pub mod reflog;
pub mod show;
pub mod status;
pub mod submodule;

pub use cherry_pick::{CherryPickResult, parse_cherry_pick};
pub use commit::{CommitResult, parse_commit};
pub use diff::{DiffEntry, DiffKind, parse_diff_name_status};
pub use log::{CommitEntry, LOG_FORMAT, parse_log};
pub use ls_tree::{TreeEntry, TreeObjectType, parse_ls_tree, parse_ls_tree_name_only};
pub use merge::{MergeResult, parse_merge};
pub use notes::parse_notes_list;
pub use pull::{PullResult, parse_pull};
pub use rebase::{RebaseResult, parse_rebase};
pub use reflog::{REFLOG_FORMAT, ReflogEntry, parse_reflog};
pub use show::{ShowResult, parse_show};
pub use status::{Status, StatusEntry, StatusKind, parse_full_status, parse_status};
pub use submodule::{SubmoduleEntry, SubmoduleStatus, parse_submodule_status};
