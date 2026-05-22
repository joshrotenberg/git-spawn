//! Integration tests for advanced (Phase 4) commands.

use git_spawn::command::config::ConfigScope;
use git_spawn::{
    BisectCommand, ConfigCommand, GitCommand, ReflogCommand, Repository, SubmoduleCommand,
    WorktreeCommand,
};

fn configure_identity(repo: &Repository) {
    for (k, v) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
        ("core.autocrlf", "false"),
    ] {
        let status = std::process::Command::new("git")
            .args(["config", "--local", k, v])
            .current_dir(repo.path())
            .status()
            .expect("git config");
        assert!(status.success());
    }
}

async fn seed_repo() -> (tempfile::TempDir, Repository) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("repo");
    std::fs::create_dir_all(&path).unwrap();
    let mut init = git_spawn::InitCommand::in_directory(&path);
    init.initial_branch("main").quiet();
    let repo = init.execute().await.expect("init");
    configure_identity(&repo);
    std::fs::write(repo.path().join("a.txt"), "one\n").unwrap();
    repo.add().path("a.txt").execute().await.unwrap();
    repo.commit().message("c1").execute().await.unwrap();
    (tmp, repo)
}

#[tokio::test]
async fn config_set_and_get() {
    let (_tmp, repo) = seed_repo().await;
    repo.config(ConfigCommand::set("test.key", "hello").scope(ConfigScope::Local))
        .execute()
        .await
        .unwrap();

    let value = repo
        .config(ConfigCommand::get("test.key").scope(ConfigScope::Local))
        .execute_value()
        .await
        .unwrap();
    assert_eq!(value, "hello");
}

#[tokio::test]
async fn grep_matches_content() {
    let (_tmp, repo) = seed_repo().await;
    std::fs::write(repo.path().join("greeting.txt"), "hello world\n").unwrap();
    repo.add().path("greeting.txt").execute().await.unwrap();
    repo.commit().message("greet").execute().await.unwrap();

    let out = repo
        .grep("hello")
        .fixed_strings()
        .line_number()
        .execute()
        .await
        .unwrap();
    assert!(out.stdout.contains("greeting.txt"));
    assert!(out.stdout.contains("hello world"));
}

#[tokio::test]
async fn grep_no_match_errors() {
    // git grep exits 1 when nothing matches.
    let (_tmp, repo) = seed_repo().await;
    let err = repo
        .grep("nothing-matches-this-xyz")
        .fixed_strings()
        .execute()
        .await
        .unwrap_err();
    assert!(matches!(err, git_spawn::Error::CommandFailed { .. }));
}

#[tokio::test]
async fn reflog_shows_initial_commit() {
    let (_tmp, repo) = seed_repo().await;
    let out = repo
        .reflog(ReflogCommand::show().max_count(10))
        .execute()
        .await
        .unwrap();
    assert!(out.stdout.contains("c1"));
}

#[tokio::test]
async fn cherry_pick_brings_commit_forward() {
    let (_tmp, repo) = seed_repo().await;

    // Branch topic, add a second commit, then cherry-pick it back onto main.
    repo.switch().create("topic").execute().await.unwrap();
    std::fs::write(repo.path().join("b.txt"), "two\n").unwrap();
    repo.add().path("b.txt").execute().await.unwrap();
    repo.commit().message("add-b").execute().await.unwrap();

    let topic_head = {
        let mut rp = git_spawn::RevParseCommand::new();
        rp.current_dir(repo.path()).arg_str("HEAD");
        rp.execute().await.unwrap()
    };

    repo.switch().target("main").execute().await.unwrap();
    repo.cherry_pick()
        .commit(&topic_head)
        .execute()
        .await
        .unwrap();
    assert!(repo.path().join("b.txt").exists());
}

#[tokio::test]
async fn worktree_add_and_list_and_remove() {
    let tmp = tempfile::tempdir().unwrap();
    let (_tmp, repo) = {
        let path = tmp.path().join("main-repo");
        std::fs::create_dir_all(&path).unwrap();
        let mut init = git_spawn::InitCommand::in_directory(&path);
        init.initial_branch("main").quiet();
        let r = init.execute().await.unwrap();
        configure_identity(&r);
        std::fs::write(r.path().join("x"), "x").unwrap();
        r.add().path("x").execute().await.unwrap();
        r.commit().message("init").execute().await.unwrap();
        (tmp, r)
    };

    let wt_path = repo.path().parent().unwrap().join("wt");
    repo.worktree(WorktreeCommand::add(&wt_path).new_branch("wt-branch"))
        .execute()
        .await
        .unwrap();
    assert!(wt_path.join("x").exists());

    let list = repo
        .worktree(WorktreeCommand::list_porcelain())
        .execute()
        .await
        .unwrap();
    assert!(list.stdout.contains("worktree "));

    repo.worktree(WorktreeCommand::remove(&wt_path).force())
        .execute()
        .await
        .unwrap();
    assert!(!wt_path.exists());
}

#[tokio::test]
async fn submodule_status_on_empty_repo() {
    // `submodule status` on a repo without submodules returns empty stdout
    // with exit 0. Just confirm it doesn't crash.
    let (_tmp, repo) = seed_repo().await;
    let out = repo
        .submodule(SubmoduleCommand::status())
        .execute()
        .await
        .unwrap();
    assert!(out.stdout.trim().is_empty());
}

#[tokio::test]
async fn bisect_start_and_reset() {
    let (_tmp, repo) = seed_repo().await;
    // Create a second commit so bisect has somewhere to walk.
    std::fs::write(repo.path().join("a.txt"), "two\n").unwrap();
    repo.add().path("a.txt").execute().await.unwrap();
    repo.commit().message("c2").execute().await.unwrap();

    // Start bisect with explicit bad=HEAD, good=HEAD~1.
    repo.bisect(
        BisectCommand::start()
            .bad_commit("HEAD")
            .good_commit("HEAD~1"),
    )
    .execute()
    .await
    .unwrap();

    // Reset cleans up the bisect state.
    repo.bisect(BisectCommand::reset(None))
        .execute()
        .await
        .unwrap();
}
