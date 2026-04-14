//! Print a short summary of working-tree status, grouped by change kind.
//!
//! Run with:
//!
//! ```bash
//! cargo run --example status_summary -- .
//! cargo run --example status_summary -- /path/to/repo
//! ```

use git_wrapper::command::status::StatusFormat;
use git_wrapper::parse::{StatusKind, parse_status};
use git_wrapper::{GitCommand, Repository};
use std::collections::BTreeMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let repo = Repository::open(&path)?;

    let out = repo
        .status()
        .format(StatusFormat::PorcelainV1)
        .null_terminate()
        .execute()
        .await?;

    let mut buckets: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for entry in parse_status(&out.stdout)? {
        let label = match (entry.index, entry.worktree) {
            (StatusKind::Untracked, _) | (_, StatusKind::Untracked) => "untracked",
            (StatusKind::Added, _) => "staged-added",
            (_, StatusKind::Modified) => "modified",
            (StatusKind::Modified, _) => "staged-modified",
            (StatusKind::Deleted, _) | (_, StatusKind::Deleted) => "deleted",
            (StatusKind::Renamed, _) => "renamed",
            _ => "other",
        };
        buckets.entry(label).or_default().push(entry.path);
    }

    if buckets.is_empty() {
        println!("clean: no changes");
        return Ok(());
    }

    for (label, paths) in buckets {
        println!("{label} ({}):", paths.len());
        for p in paths {
            println!("  {p}");
        }
    }
    Ok(())
}
