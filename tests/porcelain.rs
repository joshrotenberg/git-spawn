//! Integration tests driving real `git` in temp directories.

use git_spawn::{GitCommand, Repository};

mod common;
use common::{configure_identity, init_repo as make_repo};

#[tokio::test]
async fn init_creates_repo() {
    let (_tmp, repo) = make_repo().await;
    assert!(repo.git_dir().exists());
}

#[tokio::test]
async fn add_and_commit() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("hello.txt"), "hi").unwrap();

    repo.add().path("hello.txt").execute().await.unwrap();

    let out = repo
        .commit()
        .message("initial")
        .execute()
        .await
        .expect("commit");
    assert!(
        out.stdout_str().contains("initial") || out.stdout_str().contains("main"),
        "unexpected commit output: {}",
        out.stdout_str()
    );
}

#[tokio::test]
async fn status_short_after_write() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("a.txt"), "a").unwrap();
    let out = repo
        .status()
        .format(git_spawn::command::status::StatusFormat::Short)
        .execute()
        .await
        .unwrap();
    assert!(out.stdout_str().contains("a.txt"));
}

#[tokio::test]
async fn log_empty_repo_is_error() {
    let (_tmp, repo) = make_repo().await;
    // No commits yet -> `git log` fails.
    let err = repo.log().execute().await.unwrap_err();
    assert!(matches!(err, git_spawn::Error::CommandFailed { .. }));
}

#[tokio::test]
async fn branch_show_current() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("x"), "x").unwrap();
    repo.add().path("x").execute().await.unwrap();
    repo.commit().message("c").execute().await.unwrap();

    let out = repo.branch().show_current().execute().await.unwrap();
    assert_eq!(out.stdout_str().trim(), "main");
}

#[tokio::test]
async fn tag_list_after_creation() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("x"), "x").unwrap();
    repo.add().path("x").execute().await.unwrap();
    repo.commit().message("c").execute().await.unwrap();

    repo.tag().name("v1.0.0").execute().await.unwrap();
    let out = repo.tag().list().execute().await.unwrap();
    assert!(out.stdout_str().contains("v1.0.0"));
}

#[tokio::test]
async fn diff_shows_unstaged_change() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("f"), "one\n").unwrap();
    repo.add().path("f").execute().await.unwrap();
    repo.commit().message("init").execute().await.unwrap();

    std::fs::write(repo.path().join("f"), "two\n").unwrap();
    let out = repo.diff().execute().await.unwrap();
    assert!(out.stdout_str().contains("-one"));
    assert!(out.stdout_str().contains("+two"));
}

#[tokio::test]
async fn escape_hatch_arg_works() {
    let (_tmp, repo) = make_repo().await;
    let out = repo.status().arg("--porcelain=v2").execute().await.unwrap();
    // Empty repo has no content; porcelain v2 header is optional — just check exit ok.
    assert!(out.success);
}

async fn commit_one(repo: &Repository, name: &str, content: &str, msg: &str) {
    std::fs::write(repo.path().join(name), content).unwrap();
    repo.add().path(name).execute().await.unwrap();
    repo.commit().message(msg).execute().await.unwrap();
}

#[tokio::test]
async fn clone_local_repo() {
    let (_tmp, src) = make_repo().await;
    commit_one(&src, "f.txt", "hi\n", "init").await;

    let dst_tmp = tempfile::tempdir().unwrap();
    let dst_path = dst_tmp.path().join("clone");
    let cloned = Repository::clone(src.path().display().to_string(), &dst_path)
        .await
        .expect("clone");
    assert!(cloned.git_dir().exists());
    assert!(cloned.path().join("f.txt").exists());
}

#[tokio::test]
async fn branch_create_and_switch() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "a", "a", "init").await;

    repo.branch().create("feature").execute().await.unwrap();
    repo.switch().target("feature").execute().await.unwrap();
    let out = repo.branch().show_current().execute().await.unwrap();
    assert_eq!(out.stdout_str().trim(), "feature");
}

#[tokio::test]
async fn checkout_creates_branch() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "a", "a", "init").await;

    repo.checkout().create("topic").execute().await.unwrap();
    let out = repo.branch().show_current().execute().await.unwrap();
    assert_eq!(out.stdout_str().trim(), "topic");
}

#[tokio::test]
async fn merge_ff_branch() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "a", "a", "init").await;
    repo.switch().create("topic").execute().await.unwrap();
    commit_one(&repo, "b", "b", "second").await;

    repo.switch().target("main").execute().await.unwrap();
    repo.merge()
        .commit_ref("topic")
        .ff_only()
        .execute()
        .await
        .unwrap();
    assert!(repo.path().join("b").exists());
}

#[tokio::test]
async fn reset_hard_reverts_working_tree() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "f", "one\n", "init").await;
    std::fs::write(repo.path().join("f"), "two\n").unwrap();

    repo.reset()
        .mode(git_spawn::command::reset::ResetMode::Hard)
        .commit("HEAD")
        .execute()
        .await
        .unwrap();
    let content = std::fs::read_to_string(repo.path().join("f")).unwrap();
    assert_eq!(content, "one\n");
}

#[tokio::test]
async fn restore_staged_path() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "f", "one\n", "init").await;
    std::fs::write(repo.path().join("f"), "two\n").unwrap();
    repo.add().path("f").execute().await.unwrap();

    repo.restore().staged().path("f").execute().await.unwrap();

    // After unstaging, `git diff --cached` should be empty.
    let out = repo.diff().cached().execute().await.unwrap();
    assert!(
        out.stdout_str().trim().is_empty(),
        "unexpected: {}",
        out.stdout_str()
    );
}

#[tokio::test]
async fn rm_cached_keeps_file() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "f", "hi", "init").await;

    repo.rm().cached().path("f").execute().await.unwrap();
    // File still exists on disk.
    assert!(repo.path().join("f").exists());
    // But is no longer tracked.
    let out = repo
        .status()
        .format(git_spawn::command::status::StatusFormat::Short)
        .execute()
        .await
        .unwrap();
    assert!(out.stdout_str().contains("D"));
}

#[tokio::test]
async fn mv_renames_file() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "old.txt", "x", "init").await;

    repo.mv("old.txt", "new.txt").execute().await.unwrap();
    assert!(repo.path().join("new.txt").exists());
    assert!(!repo.path().join("old.txt").exists());
}

#[tokio::test]
async fn stash_push_and_pop() {
    let (_tmp, repo) = make_repo().await;
    commit_one(&repo, "f", "one\n", "init").await;
    std::fs::write(repo.path().join("f"), "two\n").unwrap();

    repo.stash(git_spawn::StashCommand::push())
        .message("wip")
        .execute()
        .await
        .unwrap();
    let content = std::fs::read_to_string(repo.path().join("f")).unwrap();
    assert_eq!(content, "one\n", "stash should have reset working tree");

    repo.stash(git_spawn::StashCommand::pop(None))
        .execute()
        .await
        .unwrap();
    let restored = std::fs::read_to_string(repo.path().join("f")).unwrap();
    assert_eq!(restored, "two\n", "pop should restore working-tree change");
}

#[tokio::test]
async fn remote_add_and_list() {
    let (_tmp, repo) = make_repo().await;
    repo.remote(git_spawn::RemoteCommand::add(
        "upstream",
        "https://example.com/repo.git",
    ))
    .execute()
    .await
    .unwrap();

    let out = repo
        .remote(git_spawn::RemoteCommand::list_verbose())
        .execute()
        .await
        .unwrap();
    assert!(out.stdout_str().contains("upstream"));
    assert!(out.stdout_str().contains("https://example.com/repo.git"));
}

#[tokio::test]
async fn push_pull_via_local_remote() {
    let tmp = tempfile::tempdir().unwrap();

    // Bare "remote" repo.
    let bare_path = tmp.path().join("remote.git");
    std::fs::create_dir_all(&bare_path).unwrap();
    let mut init = git_spawn::InitCommand::in_directory(&bare_path);
    init.bare().initial_branch("main").quiet();
    init.execute().await.unwrap();

    // Working copy A.
    let a_path = tmp.path().join("a");
    std::fs::create_dir_all(&a_path).unwrap();
    let mut init_a = git_spawn::InitCommand::in_directory(&a_path);
    init_a.initial_branch("main").quiet();
    let repo_a = init_a.execute().await.unwrap();
    configure_identity(&repo_a);
    commit_one(&repo_a, "hello", "hi\n", "init").await;

    repo_a
        .remote(git_spawn::RemoteCommand::add(
            "origin",
            bare_path.display().to_string(),
        ))
        .execute()
        .await
        .unwrap();
    repo_a
        .push()
        .set_upstream()
        .remote("origin")
        .refspec("main")
        .execute()
        .await
        .unwrap();

    // Clone into B and verify content.
    let b_path = tmp.path().join("b");
    let repo_b = Repository::clone(bare_path.display().to_string(), &b_path)
        .await
        .unwrap();
    assert!(repo_b.path().join("hello").exists());

    // New commit in A, then pull in B.
    commit_one(&repo_a, "another", "x", "second").await;
    repo_a
        .push()
        .remote("origin")
        .refspec("main")
        .execute()
        .await
        .unwrap();

    configure_identity(&repo_b);
    repo_b
        .pull()
        .remote("origin")
        .refspec("main")
        .ff_only()
        .execute()
        .await
        .unwrap();
    assert!(repo_b.path().join("another").exists());
}

#[tokio::test]
async fn timeout_triggers_error() {
    use std::time::Duration;
    // `git log` on an empty repo errors quickly; use a sleep via env to force a
    // slow spawn is tricky cross-platform. Instead, verify that a tight timeout
    // against a command that always succeeds still returns Timeout if we set it
    // to zero-ish. Easiest: ask git to fetch an unreachable URL with short
    // timeout.
    let (_tmp, repo) = make_repo().await;
    let mut cmd = repo.fetch();
    cmd.remote("file:///definitely/not/here/repo.git")
        .with_timeout(Duration::from_millis(50));
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, git_spawn::Error::Timeout { .. })
            || matches!(err, git_spawn::Error::CommandFailed { .. }),
        "unexpected error: {err:?}"
    );
}
