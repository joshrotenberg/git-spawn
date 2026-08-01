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
