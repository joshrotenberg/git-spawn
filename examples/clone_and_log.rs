//! Clone a repository into a temp directory and print the last N commits.
//!
//! ```bash
//! cargo run --example clone_and_log -- https://github.com/octocat/Hello-World.git 10
//! ```

use git_wrapper::parse::{LOG_FORMAT, parse_log};
use git_wrapper::{GitCommand, Repository};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let url = args.next().ok_or("usage: clone_and_log <url> [count]")?;
    let count: u32 = args.next().map_or(10, |s| s.parse().unwrap_or(10));

    let tmp = tempfile::tempdir()?;
    let dest = tmp.path().join("clone");
    println!("cloning {url} into {}", dest.display());

    let repo = Repository::clone(url, &dest).await?;

    let out = repo
        .log()
        .max_count(count)
        .format(LOG_FORMAT)
        .execute()
        .await?;
    let commits = parse_log(&out.stdout)?;

    for c in commits {
        println!(
            "{} {} <{}> {}",
            c.short_sha, c.author_name, c.author_email, c.subject
        );
    }
    Ok(())
}
