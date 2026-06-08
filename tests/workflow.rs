//! Real-git integration tests for the `workflow` feature: `info`, `branches`,
//! `tags`, `history`, and the `workflow` compositions.

#![cfg(feature = "workflow")]

use git_spawn::{GitCommand, Repository};

mod common;
use common::{configure_identity, init_repo as make_repo};

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

// ---------- tags ----------

#[tokio::test]
async fn tags_create_and_list_lightweight() {
    use git_spawn::tags::TagKind;

    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    repo.tags().create("v0.1", "HEAD").await.expect("create");

    let tags = repo.tags().list().await.expect("list");
    assert_eq!(tags.len(), 1);
    let t = &tags[0];
    assert_eq!(t.name, "v0.1");
    assert_eq!(t.kind, TagKind::Lightweight);
    assert!(!t.target.is_empty());
    assert!(t.message.is_none());
    assert!(t.tagger.is_none());
}

#[tokio::test]
async fn tags_create_annotated_populates_message_and_tagger() {
    use git_spawn::tags::TagKind;

    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    repo.tags()
        .create_annotated("v1.0", "HEAD", "first release")
        .await
        .expect("annotated");

    let tags = repo.tags().list().await.expect("list");
    let t = tags
        .iter()
        .find(|t| t.name == "v1.0")
        .expect("v1.0 present");
    assert_eq!(t.kind, TagKind::Annotated);
    assert_eq!(t.message.as_deref(), Some("first release"));
    let tagger = t.tagger.as_ref().expect("tagger populated");
    assert_eq!(tagger.email, "test@example.com");
    assert!(!tagger.date.is_empty());
}

#[tokio::test]
async fn tags_list_matching_filters_by_pattern() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    repo.tags().create("v0.1", "HEAD").await.unwrap();
    repo.tags().create("v0.2", "HEAD").await.unwrap();
    repo.tags().create("rc-1", "HEAD").await.unwrap();

    let mut names: Vec<_> = repo
        .tags()
        .list_matching("refs/tags/v*")
        .await
        .expect("matching")
        .into_iter()
        .map(|t| t.name)
        .collect();
    names.sort();
    assert_eq!(names, vec!["v0.1", "v0.2"]);
}

#[tokio::test]
async fn tags_delete_removes_tag() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    repo.tags().create("doomed", "HEAD").await.unwrap();
    repo.tags().delete("doomed").await.expect("delete");

    let tags = repo.tags().list().await.unwrap();
    assert!(tags.iter().all(|t| t.name != "doomed"));
}

// ---------- history ----------

#[tokio::test]
async fn history_returns_single_initial_commit() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    let commits = repo.history().execute().await.expect("walk");
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].subject, "init");
    assert_eq!(commits[0].author_email, "test@example.com");
    assert!(!commits[0].sha.is_empty());
    assert!(!commits[0].short_sha.is_empty());
}

#[tokio::test]
async fn history_max_count_limits_results() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    // Add three more commits.
    for i in 1..=3 {
        std::fs::write(repo.path().join(format!("f{i}")), "x").unwrap();
        repo.add().path(format!("f{i}")).execute().await.unwrap();
        repo.commit()
            .message(format!("commit {i}"))
            .execute()
            .await
            .unwrap();
    }

    let commits = repo.history().max_count(2).execute().await.expect("walk");
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "commit 3");
    assert_eq!(commits[1].subject, "commit 2");
}

#[tokio::test]
async fn history_filter_by_author_and_grep() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    std::fs::write(repo.path().join("a"), "1").unwrap();
    repo.add().path("a").execute().await.unwrap();
    repo.commit()
        .message("feat: thing one")
        .arg("--author=Alice <alice@example.com>")
        .execute()
        .await
        .unwrap();

    std::fs::write(repo.path().join("b"), "2").unwrap();
    repo.add().path("b").execute().await.unwrap();
    repo.commit()
        .message("fix: thing two")
        .arg("--author=Bob <bob@example.com>")
        .execute()
        .await
        .unwrap();

    let alice = repo
        .history()
        .author("Alice")
        .execute()
        .await
        .expect("by author");
    assert_eq!(alice.len(), 1);
    assert_eq!(alice[0].subject, "feat: thing one");

    let fixes = repo
        .history()
        .grep("^fix:")
        .execute()
        .await
        .expect("by grep");
    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].subject, "fix: thing two");
}

#[tokio::test]
async fn history_reverse_returns_oldest_first() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;
    std::fs::write(repo.path().join("a"), "x").unwrap();
    repo.add().path("a").execute().await.unwrap();
    repo.commit().message("second").execute().await.unwrap();

    let commits = repo.history().reverse().execute().await.expect("rev");
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "init");
    assert_eq!(commits[1].subject, "second");
}

// ---------- workflow ----------

#[tokio::test]
async fn workflow_feature_branch_creates_and_switches() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    repo.workflow()
        .feature_branch("feature/x", "main")
        .await
        .expect("feature_branch");

    let info = repo.info().await.unwrap();
    assert_eq!(info.branch.as_deref(), Some("feature/x"));
}

#[tokio::test]
async fn workflow_commit_all_stages_and_commits() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    std::fs::write(repo.path().join("untracked"), "hi").unwrap();
    std::fs::write(repo.path().join("README"), "changed").unwrap();

    repo.workflow()
        .commit_all("bulk: snapshot")
        .await
        .expect("commit_all");

    let info = repo.info().await.unwrap();
    assert!(!info.dirty, "expected clean tree after commit_all");

    let commits = repo.history().max_count(1).execute().await.unwrap();
    assert_eq!(commits[0].subject, "bulk: snapshot");
}

#[tokio::test]
async fn workflow_squash_merge_stages_without_committing() {
    let (_tmp, repo) = make_repo().await;
    make_initial_commit(&repo).await;

    // Branch off main, add a commit on the branch.
    repo.workflow()
        .feature_branch("feature/y", "main")
        .await
        .unwrap();
    std::fs::write(repo.path().join("y.txt"), "y").unwrap();
    repo.add().path("y.txt").execute().await.unwrap();
    repo.commit().message("on feature").execute().await.unwrap();

    // Back to main, squash-merge the feature branch.
    repo.checkout().target("main").execute().await.unwrap();
    repo.workflow()
        .squash_merge("feature/y")
        .await
        .expect("squash_merge");

    // The feature change is staged on main but not yet committed.
    let info = repo.info().await.unwrap();
    assert_eq!(info.branch.as_deref(), Some("main"));
    assert!(
        info.dirty,
        "squash leaves changes staged for the user to commit"
    );

    let head_subject = repo
        .history()
        .max_count(1)
        .execute()
        .await
        .unwrap()
        .into_iter()
        .next()
        .map(|c| c.subject)
        .unwrap_or_default();
    assert_eq!(head_subject, "init", "no merge commit yet");
}

#[tokio::test]
async fn workflow_sync_rebases_against_upstream() {
    // Set up bare remote + working clone, push so main has an upstream.
    let tmp = tempfile::tempdir().unwrap();
    let bare = tmp.path().join("remote.git");
    let work = tmp.path().join("work");
    let status = std::process::Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&bare)
        .status()
        .unwrap();
    assert!(status.success());

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
        .unwrap();

    // sync() should be a no-op against an in-sync upstream and not error.
    repo.workflow().sync().await.expect("sync");

    let info = repo.info().await.unwrap();
    assert_eq!(info.ahead, 0);
    assert_eq!(info.behind, 0);
}
