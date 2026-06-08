//! Tally commits per author in a repo and print the top contributors.
//!
//! ```bash
//! cargo run --example commit_counter -- /path/to/repo 20
//! ```

use git_spawn::parse::{LOG_FORMAT, parse_log};
use git_spawn::{GitCommand, Repository};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| ".".into());
    let top_n: usize = args.next().map_or(20, |s| s.parse().unwrap_or(20));

    let repo = Repository::open(&path)?;
    let out = repo.log().format(LOG_FORMAT).execute().await?;
    let commits = parse_log(&out.stdout_str())?;

    let mut counts: HashMap<(String, String), u32> = HashMap::new();
    for c in &commits {
        *counts
            .entry((c.author_name.clone(), c.author_email.clone()))
            .or_default() += 1;
    }

    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.1));

    println!("{} total commits", commits.len());
    println!("top {top_n} authors:");
    for ((name, email), count) in sorted.into_iter().take(top_n) {
        println!("  {count:>6}  {name} <{email}>");
    }
    Ok(())
}
