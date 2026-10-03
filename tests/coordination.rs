//! Repository coordination queries exercised against real Git.
use git_spawn::command::config::{ConfigCommand, ConfigScope};
use git_spawn::{DiffCommand, Error, GitCommand, Repository};
mod common;

#[tokio::test]
async fn common_directory_query_supports_bare_repositories() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bare.git");
    git_spawn::InitCommand::in_directory(&path)
        .bare()
        .execute()
        .await
        .unwrap();
    let repo = Repository::open(&path).unwrap();
    let actual = repo
        .rev_parse()
        .absolute_git_common_dir()
        .execute()
        .await
        .unwrap();
    assert_eq!(
        std::fs::canonicalize(actual).unwrap(),
        std::fs::canonicalize(path).unwrap()
    );
}

async fn committed_repo() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;
    std::fs::write(repo.path().join("tracked"), "original\n").unwrap();
    repo.add().path("tracked").execute().await.unwrap();
    repo.commit().message("initial").execute().await.unwrap();
    (tmp, repo)
}

#[tokio::test]
async fn diff_predicate_distinguishes_worktree_and_index() {
    let (_tmp, repo) = committed_repo().await;
    let command = repo.diff();
    assert!(!command.has_changes().await.unwrap());
    std::fs::write(repo.path().join("tracked"), "modified\n").unwrap();
    assert!(command.has_changes().await.unwrap());
    assert!(!repo.diff().cached().has_changes().await.unwrap());
    repo.add().path("tracked").execute().await.unwrap();
    assert!(!command.has_changes().await.unwrap());
    assert!(repo.diff().cached().has_changes().await.unwrap());
    assert!(!command.quiet, "the predicate must not mutate its builder");
}

#[tokio::test]
async fn diff_predicate_respects_revisions_and_pathspecs() {
    let (_tmp, repo) = committed_repo().await;
    std::fs::write(repo.path().join("tracked"), "second\n").unwrap();
    repo.add().path("tracked").execute().await.unwrap();
    repo.commit().message("second").execute().await.unwrap();
    assert!(
        repo.diff()
            .revision("HEAD~1")
            .revision("HEAD")
            .has_changes()
            .await
            .unwrap()
    );
    assert!(
        !repo
            .diff()
            .revision("HEAD")
            .revision("HEAD")
            .has_changes()
            .await
            .unwrap()
    );
    assert!(
        !repo
            .diff()
            .revision("HEAD~1")
            .revision("HEAD")
            .path("other")
            .has_changes()
            .await
            .unwrap()
    );
    assert!(
        repo.diff()
            .revision("HEAD~1")
            .revision("HEAD")
            .path("tracked")
            .has_changes()
            .await
            .unwrap()
    );
    std::fs::write(repo.path().join("tracked"), "third\n").unwrap();
    assert!(!repo.diff().path("other").has_changes().await.unwrap());
    assert!(repo.diff().path("tracked").has_changes().await.unwrap());
}

#[tokio::test]
async fn diff_predicate_does_not_count_untracked_files() {
    let (_tmp, repo) = committed_repo().await;
    std::fs::write(repo.path().join("untracked"), "new\n").unwrap();
    assert!(!repo.diff().has_changes().await.unwrap());
}

#[tokio::test]
async fn diff_predicate_preserves_git_errors_and_spawn_failures() {
    let (_tmp, repo) = committed_repo().await;
    let err = repo
        .diff()
        .revision("missing-revision")
        .has_changes()
        .await
        .unwrap_err();
    match err {
        Error::CommandFailed {
            exit_code, stderr, ..
        } => {
            assert!(exit_code > 1);
            assert!(!stderr.is_empty());
        }
        other => panic!("expected Git failure: {other:?}"),
    }
    let err = DiffCommand::new()
        .current_dir(repo.path().join("missing-directory"))
        .has_changes()
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Io { .. }));
}

#[tokio::test]
async fn diff_predicate_preserves_executor_cancellation() {
    let (_tmp, repo) = committed_repo().await;
    let token = git_spawn::CancellationToken::new();
    token.cancel();
    let err = repo
        .diff()
        .cancellation_token(token)
        .has_changes()
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Execution { .. }));
}

async fn local_config(repo: &Repository, key: &str, value: &str, append: bool) {
    let mut cmd = if append {
        ConfigCommand::add(key, value)
    } else {
        ConfigCommand::set(key, value)
    };
    cmd.scope(ConfigScope::Local);
    repo.config(cmd).execute().await.unwrap();
}

#[tokio::test]
async fn remote_get_url_applies_fetch_and_push_rewrites_and_all_urls() {
    let (_tmp, repo) = common::init_repo().await;
    repo.remote(git_spawn::RemoteCommand::add("origin", "short:repo.git"))
        .execute()
        .await
        .unwrap();
    local_config(
        &repo,
        "url.https://fetch.example/.insteadOf",
        "short:",
        false,
    )
    .await;
    local_config(
        &repo,
        "url.ssh://push.example/.pushInsteadOf",
        "short:",
        false,
    )
    .await;
    local_config(&repo, "remote.origin.url", "short:mirror.git", true).await;
    let fetch = repo
        .remote(git_spawn::RemoteCommand::get_url("origin"))
        .all()
        .execute()
        .await
        .unwrap();
    assert_eq!(
        fetch.stdout_str(),
        "https://fetch.example/repo.git\nhttps://fetch.example/mirror.git\n"
    );
    let push = repo
        .remote(git_spawn::RemoteCommand::get_url("origin"))
        .push_url()
        .all()
        .execute()
        .await
        .unwrap();
    assert_eq!(
        push.stdout_str(),
        "ssh://push.example/repo.git\nssh://push.example/mirror.git\n"
    );
    // Explicit push URLs override pushInsteadOf, while insteadOf still applies.
    local_config(&repo, "remote.origin.pushurl", "short:explicit.git", false).await;
    local_config(
        &repo,
        "remote.origin.pushurl",
        "short:explicit-mirror.git",
        true,
    )
    .await;
    let push = repo
        .remote(git_spawn::RemoteCommand::get_url("origin"))
        .push_url()
        .all()
        .execute()
        .await
        .unwrap();
    assert_eq!(
        push.stdout_str(),
        "https://fetch.example/explicit.git\nhttps://fetch.example/explicit-mirror.git\n"
    );
}

#[tokio::test]
async fn remote_get_url_selects_the_longest_rewrite_prefix() {
    let (_tmp, repo) = common::init_repo().await;
    repo.remote(git_spawn::RemoteCommand::add(
        "origin",
        "short:team/repo.git",
    ))
    .execute()
    .await
    .unwrap();
    local_config(
        &repo,
        "url.https://general.example/.insteadOf",
        "short:",
        false,
    )
    .await;
    local_config(
        &repo,
        "url.https://team.example/.insteadOf",
        "short:team/",
        false,
    )
    .await;
    let url = repo
        .remote(git_spawn::RemoteCommand::get_url("origin"))
        .execute()
        .await
        .unwrap();
    assert_eq!(url.stdout_str(), "https://team.example/repo.git\n");
}
