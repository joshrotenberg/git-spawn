//! Real-git integration tests for the `workflow` feature: `info`, `branches`.

#![cfg(feature = "workflow")]

use git_wrapper::{GitCommand, Repository};

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
        assert!(status.success(), "git config {k} failed");
    }
}

async fn make_repo() -> (tempfile::TempDir, Repository) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("repo");
    let mut init = git_wrapper::InitCommand::in_directory(&path);
    init.initial_branch("main").quiet();
    std::fs::create_dir_all(&path).unwrap();
    let repo = init.execute().await.expect("init");
    configure_identity(&repo);
    (tmp, repo)
}

async fn make_initial_commit(repo: &Repository) {
    std::fs::write(repo.path().join("README"), "init").unwrap();
    repo.add().path("README").execute().await.unwrap();
    repo.commit().message("init").execute().await.unwrap();
}

// ---------- info ----------

#[tokio::test]
async fn info_on_fresh_repo_no_remote() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    let info = repo.info().await.expect("info");
    assert_eq!(info.branch.as_deref(), Some("main"));
    assert!(info.upstream.is_none());
    assert!(info.default_branch.is_none(), "no origin remote yet");
    assert!(!info.dirty);
    assert_eq!(info.ahead, 0);
    assert_eq!(info.behind, 0);
}

#[tokio::test]
async fn info_reports_dirty_on_untracked() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;
    std::fs::write(repo.path().join("new.txt"), "x").unwrap();

    let info = repo.info().await.expect("info");
    assert!(info.dirty, "expected dirty with untracked file");
}

#[tokio::test]
async fn info_reports_dirty_on_modified() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;
    std::fs::write(repo.path().join("README"), "changed").unwrap();

    let info = repo.info().await.expect("info");
    assert!(info.dirty, "expected dirty with modified file");
}

#[tokio::test]
async fn info_with_upstream_and_default_branch() {
    // bare "remote" + a clone. After commit + push the cloned repo has both
    // an upstream and a populated refs/remotes/origin/HEAD.
    let tmp = tempfile::tempdir().unwrap();
    let bare = tmp.path().join("remote.git");
    let work = tmp.path().join("work");

    // Initialize a bare repo to act as remote.
    let status = std::process::Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&bare)
        .status()
        .expect("git init --bare");
    assert!(status.success());

    // Clone it into the working repo.
    let repo = Repository::clone(bare.display().to_string(), &work)
        .await
        .expect("clone");
    configure_identity(&repo);

    make_initial_commit(&repo).await;
    repo.push()
        .remote("origin")
        .refspec("main")
        .arg("-u")
        .execute()
        .await
        .expect("push");

    // Populate refs/remotes/origin/HEAD on the clone.
    let status = std::process::Command::new("git")
        .args(["remote", "set-head", "origin", "main"])
        .current_dir(repo.path())
        .status()
        .expect("git remote set-head");
    assert!(status.success());

    let info = repo.info().await.expect("info");
    assert_eq!(info.branch.as_deref(), Some("main"));
    assert_eq!(info.upstream.as_deref(), Some("origin/main"));
    assert_eq!(info.default_branch.as_deref(), Some("main"));
    assert!(!info.dirty);
    assert_eq!(info.ahead, 0);
    assert_eq!(info.behind, 0);
}

// ---------- branches ----------

#[tokio::test]
async fn branches_list_returns_main() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    let branches = repo.branches().list().await.expect("list");
    assert_eq!(branches.len(), 1);
    let b = &branches[0];
    assert_eq!(b.name, "main");
    assert!(b.current);
    assert!(b.upstream.is_none());
    assert!(!b.head.is_empty(), "head sha populated");
    assert_eq!(b.subject.as_deref(), Some("init"));
}

#[tokio::test]
async fn branches_list_multiple_marks_current() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    // Create two more branches.
    repo.branch().create("feature/a").execute().await.unwrap();
    repo.branch().create("feature/b").execute().await.unwrap();

    let mut branches = repo.branches().list().await.expect("list");
    branches.sort_by(|a, b| a.name.cmp(&b.name));
    assert_eq!(branches.len(), 3);

    let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, vec!["feature/a", "feature/b", "main"]);

    let current_count = branches.iter().filter(|b| b.current).count();
    assert_eq!(current_count, 1);
    assert!(branches.iter().find(|b| b.name == "main").unwrap().current);
}

#[tokio::test]
async fn branches_list_matching_filter() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;
    repo.branch().create("feature/a").execute().await.unwrap();
    repo.branch().create("feature/b").execute().await.unwrap();
    repo.branch().create("hotfix/x").execute().await.unwrap();

    let matched = repo
        .branches()
        .list_matching("refs/heads/feature/*")
        .await
        .expect("matching");
    let mut names: Vec<_> = matched.into_iter().map(|b| b.name).collect();
    names.sort();
    assert_eq!(names, vec!["feature/a", "feature/b"]);
}

#[tokio::test]
async fn branches_delete_merged_removes_only_merged() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    // Branch off main while everything is still merged.
    repo.branch().create("merged").execute().await.unwrap();

    // Create an unmerged branch with its own commit.
    repo.branch().create("unmerged").execute().await.unwrap();
    repo.checkout().target("unmerged").execute().await.unwrap();
    std::fs::write(repo.path().join("extra.txt"), "x").unwrap();
    repo.add().path("extra.txt").execute().await.unwrap();
    repo.commit().message("extra").execute().await.unwrap();
    repo.checkout().target("main").execute().await.unwrap();

    let deleted = repo
        .branches()
        .delete_merged("main")
        .await
        .expect("delete_merged");
    assert_eq!(deleted, vec!["merged".to_string()]);

    let remaining: Vec<String> = repo
        .branches()
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|b| b.name)
        .collect();
    assert!(remaining.contains(&"main".to_string()));
    assert!(remaining.contains(&"unmerged".to_string()));
    assert!(!remaining.contains(&"merged".to_string()));
}

#[tokio::test]
async fn branches_rename_changes_name() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;
    repo.branch().create("old-name").execute().await.unwrap();

    repo.branches()
        .rename("old-name", "new-name")
        .await
        .expect("rename");

    let names: Vec<String> = repo
        .branches()
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|b| b.name)
        .collect();
    assert!(names.contains(&"new-name".to_string()));
    assert!(!names.contains(&"old-name".to_string()));
}
