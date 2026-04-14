//! Integration tests driving real `git` in temp directories.

use git_wrapper::{GitCommand, Repository};

fn configure_identity(repo: &Repository) {
    // Configure a local identity so commits work in CI / clean envs.
    for (k, v) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test"),
        ("commit.gpgsign", "false"),
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
        out.stdout.contains("initial") || out.stdout.contains("main"),
        "unexpected commit output: {}",
        out.stdout
    );
}

#[tokio::test]
async fn status_short_after_write() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("a.txt"), "a").unwrap();
    let out = repo
        .status()
        .format(git_wrapper::command::status::StatusFormat::Short)
        .execute()
        .await
        .unwrap();
    assert!(out.stdout.contains("a.txt"));
}

#[tokio::test]
async fn log_empty_repo_is_error() {
    let (_tmp, repo) = make_repo().await;
    // No commits yet -> `git log` fails.
    let err = repo.log().execute().await.unwrap_err();
    assert!(matches!(err, git_wrapper::Error::CommandFailed { .. }));
}

#[tokio::test]
async fn branch_show_current() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("x"), "x").unwrap();
    repo.add().path("x").execute().await.unwrap();
    repo.commit().message("c").execute().await.unwrap();

    let out = repo.branch().show_current().execute().await.unwrap();
    assert_eq!(out.stdout.trim(), "main");
}

#[tokio::test]
async fn tag_list_after_creation() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("x"), "x").unwrap();
    repo.add().path("x").execute().await.unwrap();
    repo.commit().message("c").execute().await.unwrap();

    repo.tag().name("v1.0.0").execute().await.unwrap();
    let out = repo.tag().list().execute().await.unwrap();
    assert!(out.stdout.contains("v1.0.0"));
}

#[tokio::test]
async fn diff_shows_unstaged_change() {
    let (_tmp, repo) = make_repo().await;
    std::fs::write(repo.path().join("f"), "one\n").unwrap();
    repo.add().path("f").execute().await.unwrap();
    repo.commit().message("init").execute().await.unwrap();

    std::fs::write(repo.path().join("f"), "two\n").unwrap();
    let out = repo.diff().execute().await.unwrap();
    assert!(out.stdout.contains("-one"));
    assert!(out.stdout.contains("+two"));
}

#[tokio::test]
async fn escape_hatch_arg_works() {
    let (_tmp, repo) = make_repo().await;
    let out = repo.status().arg("--porcelain=v2").execute().await.unwrap();
    // Empty repo has no content; porcelain v2 header is optional — just check exit ok.
    assert!(out.success);
}
