//! Integration tests for plumbing commands and typed parsers.

use git_spawn::{
    AmCommand, ApplyCommand, CatFileCommand, CherryCommand, DescribeCommand, Error,
    ForEachRefCommand, FormatPatchCommand, GitCommand, HashObjectCommand, InterpretTrailersCommand,
    LogCommand, LsFilesCommand, LsTreeCommand, Repository, RevParseCommand, ShowRefCommand,
    SymbolicRefCommand, UpdateRefCommand, VerifyCommitCommand, VerifyTagCommand,
};

use git_spawn::command::interpret_trailers::TrailerIfExists;
use git_spawn::command::reset::ResetMode;

mod common;

async fn make_repo_with_commit() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;
    std::fs::write(repo.path().join("hello.txt"), "hi\n").unwrap();
    repo.add().path("hello.txt").execute().await.unwrap();
    repo.commit().message("init").execute().await.unwrap();
    (tmp, repo)
}

#[tokio::test]
async fn rev_parse_resolves_head() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = RevParseCommand::new();
    cmd.current_dir(repo.path()).arg_str("HEAD");
    let sha = cmd.execute().await.unwrap();
    assert_eq!(sha.len(), 40, "unexpected SHA: {sha}");
}

#[tokio::test]
async fn rev_parse_show_toplevel() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = RevParseCommand::new();
    cmd.current_dir(repo.path()).show_toplevel();
    let top = cmd.execute().await.unwrap();
    // Compare via canonicalized paths to avoid differences like /var vs /private/var on macOS.
    let want = std::fs::canonicalize(repo.path()).unwrap();
    let got = std::fs::canonicalize(&top).unwrap();
    assert_eq!(got, want);
}

#[tokio::test]
async fn ls_files_sees_tracked_file() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = LsFilesCommand::new();
    cmd.current_dir(repo.path()).cached();
    let out = cmd.execute().await.unwrap();
    assert!(out.stdout_str().lines().any(|l| l == "hello.txt"));
}

#[tokio::test]
async fn ls_tree_head_name_only() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = LsTreeCommand::new("HEAD");
    cmd.current_dir(repo.path()).name_only();
    let out = cmd.execute().await.unwrap();
    assert!(out.stdout_str().contains("hello.txt"));
}

#[tokio::test]
async fn cat_file_type_and_pretty_print() {
    let (_tmp, repo) = make_repo_with_commit().await;

    let mut t = CatFileCommand::object_type("HEAD");
    t.current_dir(repo.path());
    assert_eq!(t.execute().await.unwrap(), "commit");

    let mut p = CatFileCommand::pretty_print("HEAD:hello.txt");
    p.current_dir(repo.path());
    assert_eq!(p.execute().await.unwrap(), "hi");
}

#[tokio::test]
async fn hash_object_write_and_read_back() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let blob_path = repo.path().join("blobby.txt");
    std::fs::write(&blob_path, "some bytes\n").unwrap();

    let mut h = HashObjectCommand::new();
    h.current_dir(repo.path()).write().path(&blob_path);
    let sha = h.execute().await.unwrap();
    assert_eq!(sha.len(), 40);

    let mut c = CatFileCommand::pretty_print(&sha);
    c.current_dir(repo.path());
    assert_eq!(c.execute().await.unwrap(), "some bytes");
}

#[tokio::test]
async fn cat_file_bytes_preserves_binary_blob() {
    let (_tmp, repo) = make_repo_with_commit().await;
    // Bytes that are not valid UTF-8 (and include a NUL): lossy decoding would
    // mangle these into U+FFFD.
    let raw: &[u8] = &[0xff, 0xfe, 0x00, b'h', b'i', 0x80];
    let blob_path = repo.path().join("binary.bin");
    std::fs::write(&blob_path, raw).unwrap();

    let mut h = HashObjectCommand::new();
    h.current_dir(repo.path()).write().path(&blob_path);
    let sha = h.execute().await.unwrap();

    let mut c = CatFileCommand::pretty_print(&sha);
    c.current_dir(repo.path());
    // execute_bytes round-trips the blob byte-for-byte...
    assert_eq!(c.execute_bytes().await.unwrap(), raw);
    // ...while the lossy String path corrupts it (why execute_bytes exists).
    assert_ne!(c.execute().await.unwrap().as_bytes(), raw);
}

#[tokio::test]
async fn repository_plumbing_accessors_are_scoped() {
    let (_tmp, repo) = make_repo_with_commit().await;

    // Each accessor pre-scopes current_dir, so no manual setup is needed.
    let head = repo.rev_parse().arg_str("HEAD").execute().await.unwrap();
    assert_eq!(head.len(), 40);

    let files = repo.ls_files().execute().await.unwrap();
    assert!(files.stdout_str().lines().any(|l| l == "hello.txt"));

    let refs = repo.show_ref().execute().await.unwrap();
    assert!(refs.stdout_str().contains("refs/heads/"));

    let tree = repo.ls_tree("HEAD").name_only().execute().await.unwrap();
    assert!(tree.stdout_str().contains("hello.txt"));
}

#[tokio::test]
async fn repository_object_and_ref_accessors_are_scoped() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let blob_path = repo.path().join("blobby.txt");
    std::fs::write(&blob_path, "some bytes\n").unwrap();

    // hash-object writes the blob, cat-file reads it back.
    let sha = repo
        .hash_object()
        .write()
        .path(&blob_path)
        .execute()
        .await
        .unwrap();
    assert_eq!(sha.len(), 40);
    let blob = repo
        .cat_file(CatFileCommand::pretty_print(&sha))
        .execute()
        .await
        .unwrap();
    assert_eq!(blob, "some bytes");

    // update-ref creates a ref, for-each-ref lists it.
    let head = repo.rev_parse().arg_str("HEAD").execute().await.unwrap();
    repo.update_ref()
        .ref_name("refs/heads/shadow")
        .new_value(&head)
        .execute()
        .await
        .unwrap();
    let listed = repo
        .for_each_ref()
        .pattern("refs/heads/*")
        .format("%(refname:short)")
        .execute()
        .await
        .unwrap();
    assert!(listed.stdout_str().lines().any(|l| l == "shadow"));
}

#[tokio::test]
async fn update_ref_creates_and_deletes() {
    let (_tmp, repo) = make_repo_with_commit().await;
    // Resolve HEAD to pass as new value.
    let mut rp = RevParseCommand::new();
    rp.current_dir(repo.path()).arg_str("HEAD");
    let head = rp.execute().await.unwrap();

    let mut up = UpdateRefCommand::new();
    up.current_dir(repo.path())
        .ref_name("refs/heads/shadow")
        .new_value(&head);
    up.execute().await.unwrap();

    // Verify via for-each-ref.
    let mut fe = ForEachRefCommand::new();
    fe.current_dir(repo.path())
        .pattern("refs/heads/*")
        .format("%(refname:short)");
    let out = fe.execute().await.unwrap();
    assert!(out.stdout_str().lines().any(|l| l == "shadow"));

    // Delete and confirm.
    let mut rm = UpdateRefCommand::new();
    rm.current_dir(repo.path())
        .ref_name("refs/heads/shadow")
        .delete();
    rm.execute().await.unwrap();
    let out2 = fe.execute().await.unwrap();
    assert!(!out2.stdout_str().lines().any(|l| l == "shadow"));
}

#[tokio::test]
async fn describe_always_returns_sha_when_no_tag() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut d = DescribeCommand::new();
    d.current_dir(repo.path()).always().commit("HEAD");
    let out = d.execute().await.unwrap();
    // No tag exists, so --always falls back to an abbreviated SHA (non-empty).
    assert!(!out.is_empty());
}

#[tokio::test]
async fn describe_finds_tag() {
    let (_tmp, repo) = make_repo_with_commit().await;
    repo.tag().name("v0.1.0").execute().await.unwrap();
    let mut d = DescribeCommand::new();
    d.current_dir(repo.path()).tags();
    let out = d.execute().await.unwrap();
    assert!(out.starts_with("v0.1.0"), "unexpected describe: {out}");
}

#[tokio::test]
async fn show_ref_lists_heads() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut s = ShowRefCommand::new();
    s.current_dir(repo.path()).heads();
    let out = s.execute().await.unwrap();
    assert!(out.stdout_str().contains("refs/heads/main"));
}

#[tokio::test]
async fn symbolic_ref_reads_head() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut s = SymbolicRefCommand::read("HEAD");
    s.current_dir(repo.path());
    let target = s.execute().await.unwrap();
    assert_eq!(target, "refs/heads/main");
}

#[tokio::test]
async fn symbolic_ref_short_returns_branch_name() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut s = SymbolicRefCommand::read("HEAD");
    s.short().current_dir(repo.path());
    assert_eq!(s.execute().await.unwrap(), "main");
}

#[cfg(feature = "parse")]
mod parsers {
    use super::*;
    use git_spawn::command::status::StatusFormat;
    use git_spawn::parse::{
        DiffKind, StatusKind, TreeObjectType, parse_diff_name_status, parse_log, parse_ls_tree,
        parse_ls_tree_name_only, parse_status,
    };

    #[tokio::test]
    async fn status_parser_captures_modification() {
        let (_tmp, repo) = make_repo_with_commit().await;
        std::fs::write(repo.path().join("hello.txt"), "changed\n").unwrap();
        std::fs::write(repo.path().join("new.txt"), "fresh\n").unwrap();

        let out = repo
            .status()
            .format(StatusFormat::PorcelainV1)
            .null_terminate()
            .execute()
            .await
            .unwrap();
        let entries = parse_status(&out.stdout_str()).unwrap();

        let hello = entries.iter().find(|e| e.path == "hello.txt").unwrap();
        assert_eq!(hello.worktree, StatusKind::Modified);

        let fresh = entries.iter().find(|e| e.path == "new.txt").unwrap();
        assert_eq!(fresh.index, StatusKind::Untracked);
    }

    #[tokio::test]
    async fn log_parser_reads_structured_entries() {
        let (_tmp, repo) = make_repo_with_commit().await;
        std::fs::write(repo.path().join("second.txt"), "s").unwrap();
        repo.add().path("second.txt").execute().await.unwrap();
        repo.commit()
            .message("second commit")
            .execute()
            .await
            .unwrap();

        let out = repo
            .log()
            .format(git_spawn::parse::LOG_FORMAT)
            .execute()
            .await
            .unwrap();
        let commits = parse_log(&out.stdout_str()).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "second commit");
        assert_eq!(commits[1].subject, "init");
        assert_eq!(commits[0].author_name, "Test");
    }

    #[tokio::test]
    async fn diff_name_status_parser() {
        let (_tmp, repo) = make_repo_with_commit().await;
        std::fs::write(repo.path().join("hello.txt"), "changed\n").unwrap();
        std::fs::write(repo.path().join("brand-new.txt"), "new\n").unwrap();
        repo.add().all().execute().await.unwrap();

        let out = repo
            .diff()
            .cached()
            .name_status()
            .arg("-z")
            .execute()
            .await
            .unwrap();
        let entries = parse_diff_name_status(&out.stdout_str()).unwrap();
        assert!(
            entries
                .iter()
                .any(|e| e.kind == DiffKind::Modified && e.path == "hello.txt")
        );
        assert!(
            entries
                .iter()
                .any(|e| e.kind == DiffKind::Added && e.path == "brand-new.txt")
        );
    }

    #[tokio::test]
    async fn ls_tree_parser_reads_structured_entries() {
        let (_tmp, repo) = make_repo_with_commit().await;
        std::fs::create_dir(repo.path().join("subdir")).unwrap();
        std::fs::write(repo.path().join("subdir/nested.txt"), "nested\n").unwrap();
        repo.add().all().execute().await.unwrap();
        repo.commit().message("add subdir").execute().await.unwrap();

        let out = repo.ls_tree("HEAD").execute().await.unwrap();
        let entries = parse_ls_tree(&out.stdout_str()).unwrap();

        let hello = entries.iter().find(|e| e.path == "hello.txt").unwrap();
        assert_eq!(hello.object_type, TreeObjectType::Blob);
        assert_eq!(hello.mode, "100644");
        assert_eq!(hello.sha.len(), 40);
        assert_eq!(hello.size, None);

        let subdir = entries.iter().find(|e| e.path == "subdir").unwrap();
        assert_eq!(subdir.object_type, TreeObjectType::Tree);
    }

    #[tokio::test]
    async fn ls_tree_parser_reads_name_only_output() {
        let (_tmp, repo) = make_repo_with_commit().await;
        let out = repo.ls_tree("HEAD").name_only().execute().await.unwrap();
        let paths = parse_ls_tree_name_only(&out.stdout_str());
        assert_eq!(paths, vec!["hello.txt"]);
    }
}

#[tokio::test]
async fn format_patch_writes_one_file_per_commit() {
    let (_tmp, repo) = make_repo_with_commit().await;
    std::fs::write(repo.path().join("second.txt"), "two\n").unwrap();
    repo.add().path("second.txt").execute().await.unwrap();
    repo.commit().message("second").execute().await.unwrap();

    let out_dir = repo.path().join("patches");
    let mut cmd = FormatPatchCommand::new();
    cmd.current_dir(repo.path())
        .rev_spec("HEAD~1..HEAD")
        .output_dir(&out_dir);
    let paths = cmd.execute().await.unwrap();

    assert_eq!(paths.len(), 1, "unexpected patch list: {paths:?}");
    assert!(paths[0].exists(), "git reported a missing path: {paths:?}");
    let body = std::fs::read_to_string(&paths[0]).unwrap();
    assert!(
        body.contains("second"),
        "patch body missing subject: {body}"
    );
}

#[tokio::test]
async fn format_patch_without_rev_spec_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = FormatPatchCommand::new();
    cmd.current_dir(repo.path());
    assert!(cmd.execute().await.is_err());
}

#[tokio::test]
async fn apply_replays_a_formatted_patch() {
    let (_tmp, repo) = make_repo_with_commit().await;
    std::fs::write(repo.path().join("second.txt"), "two\n").unwrap();
    repo.add().path("second.txt").execute().await.unwrap();
    repo.commit().message("second").execute().await.unwrap();

    let out_dir = repo.path().join("patches");
    let mut fmt = FormatPatchCommand::new();
    fmt.current_dir(repo.path())
        .rev_spec("HEAD~1..HEAD")
        .output_dir(&out_dir);
    let paths = fmt.execute().await.unwrap();

    // Drop the commit so the patch is the only record of the change.
    repo.reset()
        .mode(ResetMode::Hard)
        .commit("HEAD~1")
        .execute()
        .await
        .unwrap();
    assert!(!repo.path().join("second.txt").exists());

    let mut cmd = ApplyCommand::new();
    cmd.current_dir(repo.path()).patch(&paths[0]);
    cmd.execute().await.unwrap();

    let restored = std::fs::read_to_string(repo.path().join("second.txt")).unwrap();
    assert_eq!(restored, "two\n");
}

#[tokio::test]
async fn apply_check_rejects_a_patch_that_does_not_apply() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let patch = repo.path().join("bogus.patch");
    std::fs::write(
        &patch,
        "--- a/missing.txt\n+++ b/missing.txt\n@@ -1 +1 @@\n-old\n+new\n",
    )
    .unwrap();

    let mut cmd = ApplyCommand::new();
    cmd.current_dir(repo.path()).patch(&patch).check();
    assert!(cmd.execute().await.is_err());
}

#[tokio::test]
async fn apply_without_a_patch_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = ApplyCommand::new();
    cmd.current_dir(repo.path());
    assert!(cmd.execute().await.is_err());
}

#[tokio::test]
async fn am_replays_a_formatted_patch_as_a_commit() {
    let (_tmp, repo) = make_repo_with_commit().await;
    std::fs::write(repo.path().join("second.txt"), "two\n").unwrap();
    repo.add().path("second.txt").execute().await.unwrap();
    repo.commit().message("second").execute().await.unwrap();

    let out_dir = repo.path().join("patches");
    let mut fmt = FormatPatchCommand::new();
    fmt.current_dir(repo.path())
        .rev_spec("HEAD~1..HEAD")
        .output_dir(&out_dir);
    let paths = fmt.execute().await.unwrap();

    // Drop the commit so the mailbox is the only record of the change.
    repo.reset()
        .mode(ResetMode::Hard)
        .commit("HEAD~1")
        .execute()
        .await
        .unwrap();
    assert!(!repo.path().join("second.txt").exists());

    let mut cmd = AmCommand::new();
    cmd.current_dir(repo.path()).mailbox(&paths[0]);
    cmd.execute().await.unwrap();

    let restored = std::fs::read_to_string(repo.path().join("second.txt")).unwrap();
    assert_eq!(restored, "two\n");

    // Unlike `apply`, `am` records a commit carrying the patch's subject.
    let mut log = LogCommand::new();
    log.current_dir(repo.path()).max_count(1).oneline();
    let subject = log.execute().await.unwrap().stdout_str().to_string();
    assert!(
        subject.contains("second"),
        "am did not record the patch subject: {subject}"
    );
}

#[tokio::test]
async fn am_abort_restores_the_branch() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut rev = RevParseCommand::new();
    rev.current_dir(repo.path()).arg_str("HEAD");
    let before = rev.execute().await.unwrap();

    // A mailbox whose diff touches a file that does not exist here, so `am`
    // stops mid-session and leaves the repository in an `am` state.
    let mailbox = repo.path().join("0001-bogus.patch");
    std::fs::write(
        &mailbox,
        "From 0000000000000000000000000000000000000000 Mon Sep 17 00:00:00 2001\n\
         From: Test <test@example.com>\n\
         Date: Mon, 1 Jan 2024 00:00:00 +0000\n\
         Subject: [PATCH] bogus\n\
         \n\
         ---\n\
         missing.txt | 2 +-\n\
         \n\
         diff --git a/missing.txt b/missing.txt\n\
         --- a/missing.txt\n\
         +++ b/missing.txt\n\
         @@ -1 +1 @@\n\
         -old\n\
         +new\n\
         -- \n\
         2.43.0\n\
         \n",
    )
    .unwrap();

    let mut cmd = AmCommand::new();
    cmd.current_dir(repo.path()).mailbox(&mailbox);
    assert!(cmd.execute().await.is_err(), "expected am to stop");

    let mut abort = AmCommand::new();
    abort.current_dir(repo.path()).abort();
    abort.execute().await.unwrap();

    let after = rev.execute().await.unwrap();
    assert_eq!(before, after, "am --abort did not restore HEAD");
}

#[tokio::test]
async fn am_without_a_mailbox_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = AmCommand::new();
    cmd.current_dir(repo.path());
    assert!(cmd.execute().await.is_err());
}

#[tokio::test]
async fn verify_commit_rejects_an_unsigned_commit() {
    let (_tmp, repo) = make_repo_with_commit().await;
    // The fixture commit carries no signature, so verification must fail
    // rather than report success. No signing key is needed for this
    // direction, which keeps the test portable across CI runners.
    let mut cmd = VerifyCommitCommand::new();
    cmd.current_dir(repo.path()).commit("HEAD");
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::CommandFailed { .. }),
        "expected a non-zero exit, got {err:?}"
    );
}

#[tokio::test]
async fn verify_commit_without_a_commit_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = VerifyCommitCommand::new();
    cmd.current_dir(repo.path());
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidConfig { .. }),
        "expected an invalid-config error, got {err:?}"
    );
}

#[tokio::test]
async fn verify_tag_rejects_an_unsigned_tag() {
    let (_tmp, repo) = make_repo_with_commit().await;
    repo.tag()
        .name("v0.1.0")
        .message("unsigned")
        .execute()
        .await
        .unwrap();

    let mut cmd = VerifyTagCommand::new();
    cmd.current_dir(repo.path()).tag("v0.1.0");
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::CommandFailed { .. }),
        "expected a non-zero exit, got {err:?}"
    );
}

#[tokio::test]
async fn verify_tag_without_a_tag_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = VerifyTagCommand::new();
    cmd.current_dir(repo.path());
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidConfig { .. }),
        "expected an invalid-config error, got {err:?}"
    );
}

/// Stage `file` with `content` and commit it on the current branch.
async fn commit_file(repo: &Repository, file: &str, content: &str, message: &str) {
    std::fs::write(repo.path().join(file), content).unwrap();
    repo.add().path(file).execute().await.unwrap();
    repo.commit().message(message).execute().await.unwrap();
}

#[tokio::test]
async fn cherry_marks_a_commit_missing_upstream() {
    let (_tmp, repo) = make_repo_with_commit().await;
    repo.checkout().create("feature").execute().await.unwrap();
    commit_file(&repo, "feature.txt", "feature\n", "add feature").await;

    let mut cmd = CherryCommand::new();
    cmd.current_dir(repo.path())
        .upstream("main")
        .head("feature")
        .verbose();
    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(
        stdout.starts_with("+ "),
        "expected an unapplied commit marker: {stdout}"
    );
    assert!(
        stdout.contains("add feature"),
        "-v did not include the subject: {stdout}"
    );
}

#[tokio::test]
async fn cherry_head_without_an_upstream_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = CherryCommand::new();
    cmd.current_dir(repo.path()).head("feature");
    assert!(cmd.execute().await.is_err());
}

#[tokio::test]
async fn cherry_limit_without_a_head_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = CherryCommand::new();
    cmd.current_dir(repo.path()).upstream("main").limit("v1.0");
    assert!(cmd.execute().await.is_err());
}

#[cfg(feature = "parse")]
mod cherry_parser {
    use super::*;
    use git_spawn::parse::CherryStatus;

    #[tokio::test]
    async fn entries_flip_to_upstream_once_the_patch_is_applied() {
        let (_tmp, repo) = make_repo_with_commit().await;
        repo.checkout().create("feature").execute().await.unwrap();
        commit_file(&repo, "feature.txt", "feature\n", "add feature").await;

        let mut cmd = CherryCommand::new();
        cmd.current_dir(repo.path())
            .upstream("main")
            .head("feature")
            .verbose();

        let entries = cmd.parse_entries(&cmd.execute().await.unwrap());
        assert_eq!(entries.len(), 1, "unexpected entries: {entries:?}");
        assert_eq!(entries[0].status, CherryStatus::NotUpstream);
        assert_eq!(entries[0].subject.as_deref(), Some("add feature"));

        // Apply the same patch on main; git cherry then recognizes it as an
        // equivalent commit and flips the marker. main has to move first:
        // cherry-picking onto an unchanged main reproduces the commit
        // verbatim, which makes feature an ancestor and empties the report.
        let sha = entries[0].sha.clone();
        repo.checkout().target("main").execute().await.unwrap();
        commit_file(&repo, "other.txt", "other\n", "add other").await;
        repo.cherry_pick().commit(&sha).execute().await.unwrap();

        let entries = cmd.parse_entries(&cmd.execute().await.unwrap());
        assert_eq!(entries.len(), 1, "unexpected entries: {entries:?}");
        assert_eq!(entries[0].status, CherryStatus::Upstream);
    }
}

#[tokio::test]
async fn interpret_trailers_appends_a_trailer_to_stdout() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let msg = repo.path().join("MSG");
    std::fs::write(&msg, "subject line\n\nbody text\n").unwrap();

    let mut cmd = InterpretTrailersCommand::new();
    cmd.current_dir(repo.path())
        .trailer("Signed-off-by", "A U Thor <author@example.com>")
        .file(&msg);
    let out = cmd.execute().await.unwrap();

    assert!(
        out.stdout_str()
            .contains("Signed-off-by: A U Thor <author@example.com>"),
        "trailer missing from output: {}",
        out.stdout_str()
    );
    // Without --in-place the file itself is untouched.
    let on_disk = std::fs::read_to_string(&msg).unwrap();
    assert!(!on_disk.contains("Signed-off-by"), "file was rewritten");
}

#[tokio::test]
async fn interpret_trailers_in_place_rewrites_the_file() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let msg = repo.path().join("MSG");
    std::fs::write(&msg, "subject line\n\nbody text\n").unwrap();

    let mut cmd = InterpretTrailersCommand::new();
    cmd.current_dir(repo.path())
        .in_place()
        .trailer("Reviewed-by", "R Viewer <r@example.com>")
        .file(&msg);
    cmd.execute().await.unwrap();

    let on_disk = std::fs::read_to_string(&msg).unwrap();
    assert!(
        on_disk.contains("Reviewed-by: R Viewer <r@example.com>"),
        "trailer missing from rewritten file: {on_disk}"
    );
}

#[tokio::test]
async fn interpret_trailers_parse_reports_only_existing_trailers() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let msg = repo.path().join("MSG");
    std::fs::write(
        &msg,
        "subject line\n\nbody text\n\nSigned-off-by: A U Thor <author@example.com>\n",
    )
    .unwrap();

    let mut cmd = InterpretTrailersCommand::new();
    cmd.current_dir(repo.path()).parse().file(&msg);
    let out = cmd.execute().await.unwrap();

    let stdout = out.stdout_str();
    assert!(
        stdout.contains("Signed-off-by: A U Thor <author@example.com>"),
        "existing trailer missing: {stdout}"
    );
    assert!(
        !stdout.contains("subject line") && !stdout.contains("body text"),
        "--parse should drop the message body: {stdout}"
    );
}

#[tokio::test]
async fn interpret_trailers_if_exists_do_nothing_keeps_the_original() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let msg = repo.path().join("MSG");
    std::fs::write(
        &msg,
        "subject line\n\nbody text\n\nSigned-off-by: First <first@example.com>\n",
    )
    .unwrap();

    let mut cmd = InterpretTrailersCommand::new();
    cmd.current_dir(repo.path())
        .if_exists(TrailerIfExists::DoNothing)
        .trailer("Signed-off-by", "Second <second@example.com>")
        .file(&msg);
    let out = cmd.execute().await.unwrap();

    let stdout = out.stdout_str();
    assert!(
        stdout.contains("Signed-off-by: First <first@example.com>"),
        "original trailer lost: {stdout}"
    );
    assert!(
        !stdout.contains("second@example.com"),
        "doNothing still added the trailer: {stdout}"
    );
}

#[tokio::test]
async fn interpret_trailers_without_a_file_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = InterpretTrailersCommand::new();
    cmd.current_dir(repo.path())
        .trailer("Signed-off-by", "A U Thor <author@example.com>");
    assert!(cmd.execute().await.is_err());
}
