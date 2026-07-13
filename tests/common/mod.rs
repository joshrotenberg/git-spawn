//! Shared helpers for the real-git integration tests.
//!
//! Compiled into each integration binary via `mod common;`. Not every binary
//! uses every helper, so a crate-level `dead_code` allowance keeps the unused
//! ones from tripping `-D warnings`.
#![allow(dead_code)]

use git_spawn::command::config::{ConfigCommand, ConfigScope};
use git_spawn::{GitCommand, Repository};

/// Configure a local identity and deterministic settings so commits work in
/// clean CI environments. `core.autocrlf=false` keeps Windows from rewriting
/// `\n` to `\r\n` on checkout, which would break byte-for-byte assertions.
///
/// Runs through the crate's tokio-based executor rather than a blocking
/// `std::process::Command`: these helpers are called from `#[tokio::test]`
/// async fns, and mixing blocking std child processes with tokio's async
/// SIGCHLD-driven reaper in the same runtime races on Unix (tokio's wildcard
/// `waitpid` can reap the std child's exit status first). That race showed up
/// as spurious `git bisect` non-convergence on loaded macOS CI runners.
pub async fn configure_identity(repo: &Repository) {
    for (k, v) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
        ("core.autocrlf", "false"),
    ] {
        let mut cmd = ConfigCommand::set(k, v);
        cmd.scope(ConfigScope::Local);
        cmd.current_dir(repo.path());
        cmd.execute()
            .await
            .unwrap_or_else(|e| panic!("git config {k} failed: {e}"));
    }
}

/// Create an empty repository on `main` in a fresh tempdir, with identity
/// configured. Returns the tempdir guard (dropping it deletes the repo) and
/// the [`Repository`].
pub async fn init_repo() -> (tempfile::TempDir, Repository) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("repo");
    std::fs::create_dir_all(&path).unwrap();
    let mut init = git_spawn::InitCommand::in_directory(&path);
    init.initial_branch("main").quiet();
    let repo = init.execute().await.expect("init");
    configure_identity(&repo).await;
    (tmp, repo)
}
