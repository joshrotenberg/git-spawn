mod common;

use git_spawn::{GitCommand, parse::parse_diff_name_status};

#[tokio::test]
async fn object_index_and_diff_plumbing_round_trip() {
    let (_tmp, repo) = common::init_repo().await;
    std::fs::write(repo.path().join("file.txt"), "one\n").unwrap();
    repo.add().path("file.txt").execute().await.unwrap();
    repo.commit().message("initial").execute().await.unwrap();

    let tree = repo.write_tree().execute().await.unwrap();
    assert_eq!(tree.len(), 40);
    let commits = repo
        .rev_list()
        .count()
        .revision("HEAD")
        .execute()
        .await
        .unwrap();
    assert_eq!(commits, "1");

    std::fs::write(repo.path().join("file.txt"), "two\n").unwrap();
    let out = repo
        .diff_files()
        .name_status()
        .null_terminate()
        .execute()
        .await
        .unwrap();
    let entries = parse_diff_name_status(&out.stdout_str()).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, "file.txt");

    let listing = repo.ls_tree(&tree).recurse().execute().await.unwrap();
    let rebuilt = repo.mktree().stdin(listing.stdout).execute().await.unwrap();
    assert_eq!(rebuilt, tree);
}

#[tokio::test]
async fn quiet_diff_commands_report_differences() {
    let (_tmp, repo) = common::init_repo().await;
    std::fs::write(repo.path().join("file.txt"), "one\n").unwrap();
    repo.add().path("file.txt").execute().await.unwrap();
    repo.commit().message("initial").execute().await.unwrap();

    let mut files = repo.diff_files();
    files.quiet();
    assert!(!files.execute_has_differences().await.unwrap());
    std::fs::write(repo.path().join("file.txt"), "two\n").unwrap();
    assert!(files.execute_has_differences().await.unwrap());

    let mut index = repo.diff_index();
    index.quiet().cached().tree("HEAD");
    assert!(!index.execute_has_differences().await.unwrap());
    repo.add().path("file.txt").execute().await.unwrap();
    assert!(index.execute_has_differences().await.unwrap());

    let mut invalid = repo.diff_index();
    invalid.quiet().tree("not-a-revision");
    assert!(invalid.execute_has_differences().await.is_err());
}

#[tokio::test]
async fn merge_file_returns_conflicted_output() {
    let tmp = tempfile::tempdir().unwrap();
    let current = tmp.path().join("current.txt");
    let base = tmp.path().join("base.txt");
    let other = tmp.path().join("other.txt");
    std::fs::write(&current, "ours\n").unwrap();
    std::fs::write(&base, "base\n").unwrap();
    std::fs::write(&other, "theirs\n").unwrap();

    let mut command = git_spawn::MergeFileCommand::new();
    command.stdout().current(&current).base(&base).other(&other);
    let output = command.execute().await.unwrap();
    assert_eq!(output.exit_code, 1);
    assert!(output.stdout_str().contains("<<<<<<<"));
    assert!(output.stdout_str().contains("======="));
    assert!(output.stdout_str().contains(">>>>>>>"));

    command.other(tmp.path().join("missing.txt"));
    assert!(command.execute().await.is_err());
}

#[tokio::test]
async fn merge_tree_returns_conflicted_output() {
    let (_tmp, repo) = common::init_repo().await;
    std::fs::write(repo.path().join("file.txt"), "base\n").unwrap();
    repo.add().path("file.txt").execute().await.unwrap();
    repo.commit().message("base").execute().await.unwrap();

    repo.checkout().create("ours").execute().await.unwrap();
    std::fs::write(repo.path().join("file.txt"), "ours\n").unwrap();
    repo.add().path("file.txt").execute().await.unwrap();
    repo.commit().message("ours").execute().await.unwrap();

    repo.checkout().target("main").execute().await.unwrap();
    std::fs::write(repo.path().join("file.txt"), "theirs\n").unwrap();
    repo.add().path("file.txt").execute().await.unwrap();
    repo.commit().message("theirs").execute().await.unwrap();

    let clean = repo
        .merge_tree()
        .write_tree()
        .ours("main")
        .theirs("main")
        .execute()
        .await
        .unwrap();
    assert!(clean.clean);
    assert_eq!(clean.tree.len(), 40);
    assert!(clean.conflicts.is_empty());

    let result = repo
        .merge_tree()
        .write_tree()
        .ours("ours")
        .theirs("main")
        .execute()
        .await
        .unwrap();
    assert!(!result.clean);
    assert_eq!(result.tree.len(), 40);
    assert_eq!(result.conflicts, ["file.txt"]);

    let nul_result = repo
        .merge_tree()
        .write_tree()
        .null_terminate()
        .ours("ours")
        .theirs("main")
        .execute()
        .await
        .unwrap();
    assert!(!nul_result.clean);
    assert_eq!(nul_result.tree.len(), 40);
    assert_eq!(nul_result.conflicts, ["file.txt"]);

    let explicit_base = repo
        .merge_tree()
        .write_tree()
        .base("HEAD~1")
        .ours("ours")
        .theirs("main")
        .execute()
        .await
        .unwrap();
    assert!(!explicit_base.clean);
    assert_eq!(explicit_base.conflicts, ["file.txt"]);

    let mut invalid = repo.merge_tree();
    invalid.write_tree().ours("missing").theirs("main");
    assert!(invalid.execute().await.is_err());
}
