//! Integration tests for the `git notes` wrapper, run against real `git`.

use git_spawn::command::notes::NotesCommand;
use git_spawn::{GitCommand, Repository};

mod common;

async fn make_repo_with_commit() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;
    std::fs::write(repo.path().join("hello.txt"), "hi\n").unwrap();
    repo.add().path("hello.txt").execute().await.unwrap();
    repo.commit().message("init").execute().await.unwrap();
    (tmp, repo)
}

async fn head_sha(repo: &Repository) -> String {
    repo.rev_parse().arg_str("HEAD").execute().await.unwrap()
}

#[tokio::test]
async fn add_show_append_list_round_trip() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let sha = head_sha(&repo).await;

    // add a note to HEAD in a custom namespace
    repo.notes(NotesCommand::add())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .message("reviewed")
        .execute()
        .await
        .unwrap();

    // show returns it
    let shown = repo
        .notes(NotesCommand::show())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .execute()
        .await
        .unwrap();
    assert_eq!(shown.stdout_trimmed(), "reviewed");

    // append more; show reflects it
    repo.notes(NotesCommand::append())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .message("shipped")
        .execute()
        .await
        .unwrap();
    let shown = repo
        .notes(NotesCommand::show())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .execute()
        .await
        .unwrap();
    let text = shown.stdout_str();
    assert!(text.contains("reviewed"), "missing original: {text}");
    assert!(text.contains("shipped"), "missing appended: {text}");

    // list includes the (note, object) pair for our commit
    let listed = repo
        .notes(NotesCommand::list())
        .ref_namespace("refs/notes/test")
        .execute()
        .await
        .unwrap();
    assert!(
        listed.stdout_str().lines().any(|l| l.ends_with(&sha)),
        "list missing object {sha}: {}",
        listed.stdout_str()
    );
}

#[tokio::test]
async fn binary_payload_round_trips_via_file() {
    let (_tmp, repo) = make_repo_with_commit().await;

    // A few KB of high bytes (not valid UTF-8); --no-stripspace keeps them exact.
    let payload: Vec<u8> = (128u16..256)
        .flat_map(|b| std::iter::repeat_n(b as u8, 16))
        .collect();
    let payload_path = repo.path().join("payload.bin");
    std::fs::write(&payload_path, &payload).unwrap();

    repo.notes(NotesCommand::add())
        .ref_namespace("refs/notes/blobs")
        .object("HEAD")
        .message_file(&payload_path)
        .no_stripspace()
        .execute()
        .await
        .unwrap();

    let shown = repo
        .notes(NotesCommand::show())
        .ref_namespace("refs/notes/blobs")
        .object("HEAD")
        .execute()
        .await
        .unwrap();
    assert_eq!(
        shown.stdout_bytes(),
        payload.as_slice(),
        "binary note did not round-trip byte-for-byte"
    );
}

#[tokio::test]
async fn remove_then_show_fails_cleanly() {
    let (_tmp, repo) = make_repo_with_commit().await;

    repo.notes(NotesCommand::add())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .message("temp")
        .execute()
        .await
        .unwrap();

    repo.notes(NotesCommand::remove())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .execute()
        .await
        .unwrap();

    // show on a now-missing note must surface a clean CommandFailed, not panic
    // or succeed.
    let err = repo
        .notes(NotesCommand::show())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .execute()
        .await
        .unwrap_err();
    assert!(
        matches!(err, git_spawn::Error::CommandFailed { .. }),
        "expected CommandFailed, got: {err:?}"
    );

    // --ignore-missing makes a second remove a no-op success.
    repo.notes(NotesCommand::remove())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .ignore_missing()
        .execute()
        .await
        .unwrap();
}

#[tokio::test]
async fn notes_push_fetch_between_repos() {
    let tmp = tempfile::tempdir().unwrap();

    // Bare "remote" repo.
    let bare_path = tmp.path().join("remote.git");
    std::fs::create_dir_all(&bare_path).unwrap();
    let mut init = git_spawn::InitCommand::in_directory(&bare_path);
    init.bare().initial_branch("main").quiet();
    init.execute().await.unwrap();

    // Working copy A: commit, wire origin, push main.
    let a_path = tmp.path().join("a");
    std::fs::create_dir_all(&a_path).unwrap();
    let mut init_a = git_spawn::InitCommand::in_directory(&a_path);
    init_a.initial_branch("main").quiet();
    let repo_a = init_a.execute().await.unwrap();
    common::configure_identity(&repo_a).await;
    std::fs::write(repo_a.path().join("f.txt"), "hi\n").unwrap();
    repo_a.add().path("f.txt").execute().await.unwrap();
    repo_a.commit().message("init").execute().await.unwrap();
    let sha = head_sha(&repo_a).await;

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
        .remote("origin")
        .refspec("main")
        .execute()
        .await
        .unwrap();

    // Attach a note in A and push the notes namespace via a raw refspec.
    repo_a
        .notes(NotesCommand::add())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .message("from A")
        .execute()
        .await
        .unwrap();
    repo_a
        .push()
        .remote("origin")
        .refspec("refs/notes/test:refs/notes/test")
        .execute()
        .await
        .unwrap();

    // Clone into B (gets the commit), then fetch the notes ref explicitly.
    let b_path = tmp.path().join("b");
    let repo_b = Repository::clone(bare_path.display().to_string(), &b_path)
        .await
        .unwrap();
    common::configure_identity(&repo_b).await;
    repo_b
        .fetch()
        .remote("origin")
        .refspec("refs/notes/test:refs/notes/test")
        .execute()
        .await
        .unwrap();

    // B can now read the note A wrote, keyed by the shared commit SHA.
    let shown = repo_b
        .notes(NotesCommand::show())
        .ref_namespace("refs/notes/test")
        .object(&sha)
        .execute()
        .await
        .unwrap();
    assert_eq!(shown.stdout_trimmed(), "from A");
}

#[cfg(feature = "parse")]
#[tokio::test]
async fn parse_notes_list_pairs_note_and_object() {
    use git_spawn::parse::parse_notes_list;

    let (_tmp, repo) = make_repo_with_commit().await;
    let sha = head_sha(&repo).await;

    repo.notes(NotesCommand::add())
        .ref_namespace("refs/notes/test")
        .object("HEAD")
        .message("note")
        .execute()
        .await
        .unwrap();

    let listed = repo
        .notes(NotesCommand::list())
        .ref_namespace("refs/notes/test")
        .execute()
        .await
        .unwrap();
    let pairs = parse_notes_list(&listed.stdout_str());
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].1, sha, "object SHA should match HEAD");
    assert_eq!(pairs[0].0.len(), 40, "note SHA should be a full object id");
}
