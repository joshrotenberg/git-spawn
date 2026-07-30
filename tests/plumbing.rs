//! Integration tests for plumbing commands and typed parsers.

use git_spawn::{
    AmCommand, ApplyCommand, ArchiveCommand, BlameCommand, BranchCommand, BundleCommand,
    CatFileCommand, CherryCommand, CleanCommand, DescribeCommand, Error, ForEachRefCommand,
    FormatPatchCommand, FsckCommand, GcCommand, GitCommand, HashObjectCommand,
    InterpretTrailersCommand, LogCommand, LsFilesCommand, LsRemoteCommand, LsTreeCommand,
    MaintenanceCommand, MergeBaseCommand, NameRevCommand, RangeDiffCommand, Repository,
    RerereCommand, RevParseCommand, RevertCommand, ShortlogCommand, ShowRefCommand,
    SparseCheckoutCommand, SymbolicRefCommand, UpdateRefCommand, VerifyCommitCommand,
    VerifyTagCommand,
};

use git_spawn::command::archive::ArchiveFormat;
use git_spawn::command::config::{ConfigCommand, ConfigScope};
use git_spawn::command::interpret_trailers::TrailerIfExists;
use git_spawn::command::maintenance::MaintenanceTask;
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

async fn make_repo_with_two_line_history() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;
    commit_file(&repo, "notes.txt", "one\ntwo\n", "add one and two").await;
    commit_file(&repo, "notes.txt", "one\ntwo\nthree\n", "add three").await;
    (tmp, repo)
}

#[tokio::test]
async fn blame_reports_the_author_of_each_line() {
    let (_tmp, repo) = make_repo_with_two_line_history().await;
    let mut cmd = BlameCommand::new();
    cmd.current_dir(repo.path()).file("notes.txt");

    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(stdout.contains("Test"), "no author in the report: {stdout}");
    for line in ["one", "two", "three"] {
        assert!(
            stdout.contains(line),
            "line {line} missing from the report: {stdout}"
        );
    }
}

#[tokio::test]
async fn blame_line_range_limits_the_report() {
    let (_tmp, repo) = make_repo_with_two_line_history().await;
    let mut cmd = BlameCommand::new();
    cmd.current_dir(repo.path()).file("notes.txt").lines(3, 3);

    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(
        stdout.contains("three"),
        "the requested line is missing: {stdout}"
    );
    assert!(
        !stdout.contains("one"),
        "the range did not limit the report: {stdout}"
    );
}

#[tokio::test]
async fn blame_without_a_file_is_rejected() {
    let (_tmp, repo) = make_repo_with_two_line_history().await;
    let mut cmd = BlameCommand::new();
    cmd.current_dir(repo.path());
    assert!(matches!(
        cmd.execute().await,
        Err(Error::InvalidConfig { .. })
    ));
}

#[tokio::test]
async fn blame_rejects_an_inverted_line_range() {
    let (_tmp, repo) = make_repo_with_two_line_history().await;
    let mut cmd = BlameCommand::new();
    cmd.current_dir(repo.path()).file("notes.txt").lines(3, 1);
    assert!(matches!(
        cmd.execute().await,
        Err(Error::InvalidConfig { .. })
    ));
}

#[tokio::test]
async fn blame_rejects_a_zero_start_line() {
    let (_tmp, repo) = make_repo_with_two_line_history().await;
    let mut cmd = BlameCommand::new();
    cmd.current_dir(repo.path()).file("notes.txt").lines(0, 2);
    assert!(matches!(
        cmd.execute().await,
        Err(Error::InvalidConfig { .. })
    ));
}

#[tokio::test]
async fn bundle_create_then_verify_and_list_heads() {
    let (tmp, repo) = make_repo_with_commit().await;
    let bundle = tmp.path().join("all.bundle");

    let mut create = BundleCommand::create(&bundle);
    create.current_dir(repo.path()).all().quiet();
    create.execute().await.unwrap();
    assert!(bundle.is_file(), "bundle file was not written");

    let mut verify = BundleCommand::verify(&bundle);
    verify.current_dir(repo.path());
    verify.execute().await.unwrap();

    let mut heads = BundleCommand::list_heads(&bundle);
    heads.current_dir(repo.path());
    let out = heads.execute().await.unwrap();
    assert!(
        out.stdout_str().contains("refs/heads/main"),
        "unexpected list-heads output: {}",
        out.stdout_str()
    );
}

#[tokio::test]
async fn bundle_list_heads_filters_by_ref_name() {
    let (tmp, repo) = make_repo_with_commit().await;
    let mut branch = BranchCommand::new();
    branch.current_dir(repo.path()).create("topic");
    branch.execute().await.unwrap();

    let bundle = tmp.path().join("all.bundle");
    let mut create = BundleCommand::create(&bundle);
    create.current_dir(repo.path()).all().quiet();
    create.execute().await.unwrap();

    let mut heads = BundleCommand::list_heads(&bundle);
    heads.current_dir(repo.path()).ref_name("refs/heads/topic");
    let out = heads.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(
        stdout.contains("refs/heads/topic"),
        "requested ref missing: {stdout}"
    );
    assert!(
        !stdout.contains("refs/heads/main"),
        "filter did not exclude the other ref: {stdout}"
    );
}

/// Drive `main` and `other` into a both-modified conflict on `README` with
/// `rerere.enabled` set, leaving the merge in progress so git has recorded a
/// preimage for the conflict.
async fn make_rerere_conflict() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;

    // `rerere.enabled` is what makes git record anything at all. The two
    // maintenance switches keep `commit` from detaching a background
    // `gc`/`maintenance` run, which competes for `MERGE_RR.lock` with the
    // rerere action the test is about to take.
    for (key, value) in [
        ("rerere.enabled", "true"),
        ("gc.auto", "0"),
        ("maintenance.auto", "false"),
    ] {
        let mut cfg = ConfigCommand::set(key, value);
        cfg.scope(ConfigScope::Local);
        cfg.current_dir(repo.path());
        cfg.execute()
            .await
            .unwrap_or_else(|e| panic!("git config {key} failed: {e}"));
    }

    std::fs::write(repo.path().join("README"), "base\n").unwrap();
    repo.add().path("README").execute().await.unwrap();
    repo.commit().message("init").execute().await.unwrap();

    repo.branch().create("other").execute().await.unwrap();
    repo.checkout().target("other").execute().await.unwrap();
    std::fs::write(repo.path().join("README"), "other side\n").unwrap();
    repo.add().path("README").execute().await.unwrap();
    repo.commit().message("other").execute().await.unwrap();

    repo.checkout().target("main").execute().await.unwrap();
    std::fs::write(repo.path().join("README"), "main side\n").unwrap();
    repo.add().path("README").execute().await.unwrap();
    repo.commit().message("main").execute().await.unwrap();

    // The merge fails and leaves README unmerged; the non-zero exit is
    // expected here.
    let merged = repo.merge().commit_ref("other").execute().await;
    assert!(merged.is_err(), "merge should conflict");

    (tmp, repo)
}

/// Resolve the conflict left by [`make_rerere_conflict`] and commit it, which
/// is what makes git store a postimage in the resolution cache.
async fn record_rerere_resolution(repo: &Repository) {
    std::fs::write(repo.path().join("README"), "resolved\n").unwrap();
    repo.add().path("README").execute().await.unwrap();
    repo.commit().message("merged").execute().await.unwrap();
}

/// Count the `rr-cache` entries holding a file with this name.
fn rr_cache_entries_with(repo: &Repository, file: &str) -> usize {
    let cache = repo.path().join(".git").join("rr-cache");
    std::fs::read_dir(&cache)
        .unwrap_or_else(|e| panic!("reading {}: {e}", cache.display()))
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join(file).exists())
        .count()
}

/// Wait for a stale `MERGE_RR.lock` to go away.
///
/// The commit that concludes a merge can leave the lock in place for a moment,
/// and any `rerere` action that writes `MERGE_RR` fails with "Another git
/// process seems to be running" while it is there. Seen once on the macOS CI
/// runner with git 2.55; every command this test issued has already exited, so
/// the wait is for git's own background cleanup rather than for a peer.
async fn wait_for_merge_rr_unlocked(repo: &Repository) {
    let lock = repo.path().join(".git").join("MERGE_RR.lock");
    for _ in 0..50 {
        if !lock.exists() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("{} was never released", lock.display());
}

#[tokio::test]
async fn rerere_status_lists_the_conflicted_path() {
    let (_tmp, repo) = make_rerere_conflict().await;

    let mut status = RerereCommand::status();
    status.current_dir(repo.path());
    let out = status.execute().await.unwrap();
    assert_eq!(out.stdout_str().trim(), "README");
}

#[tokio::test]
async fn rerere_diff_shows_the_conflict_against_its_preimage() {
    let (_tmp, repo) = make_rerere_conflict().await;

    let mut diff = RerereCommand::diff();
    diff.current_dir(repo.path());
    let out = diff.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(
        stdout.contains("<<<<<<<") && stdout.contains("main side"),
        "unexpected rerere diff: {stdout}"
    );
}

/// Commit `a/f`, `b/f` and `top.txt`, so a pattern set naming one directory
/// has something to leave out.
async fn make_repo_with_two_directories() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;
    for dir in ["a", "b"] {
        std::fs::create_dir(repo.path().join(dir)).unwrap();
        std::fs::write(repo.path().join(dir).join("f"), "contents\n").unwrap();
    }
    std::fs::write(repo.path().join("top.txt"), "top\n").unwrap();
    repo.add().all().execute().await.unwrap();
    repo.commit().message("files").execute().await.unwrap();
    (tmp, repo)
}

/// The pattern set `sparse-checkout list` reports, one pattern per line.
async fn sparse_patterns(repo: &Repository) -> Vec<String> {
    let mut list = SparseCheckoutCommand::list();
    list.current_dir(repo.path());
    let out = list.execute().await.expect("sparse-checkout list");
    out.stdout_str()
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

#[tokio::test]
async fn sparse_checkout_set_limits_the_working_tree_to_the_pattern() {
    let (_tmp, repo) = make_repo_with_two_directories().await;

    let mut set = SparseCheckoutCommand::set("a");
    set.current_dir(repo.path());
    set.execute().await.unwrap();

    assert_eq!(sparse_patterns(&repo).await, vec!["a"]);
    assert!(repo.path().join("a").join("f").exists(), "a/f was dropped");
    assert!(!repo.path().join("b").join("f").exists(), "b/f was kept");
    // Cone mode always keeps the files at the top level.
    assert!(repo.path().join("top.txt").exists(), "top.txt was dropped");
}

#[tokio::test]
async fn sparse_checkout_add_extends_the_pattern_set() {
    let (_tmp, repo) = make_repo_with_two_directories().await;
    let mut set = SparseCheckoutCommand::set("a");
    set.current_dir(repo.path());
    set.execute().await.unwrap();

    let mut add = SparseCheckoutCommand::add("b");
    add.current_dir(repo.path());
    add.execute().await.unwrap();

    assert_eq!(sparse_patterns(&repo).await, vec!["a", "b"]);
    assert!(
        repo.path().join("b").join("f").exists(),
        "b/f was not added"
    );
}

#[tokio::test]
async fn bundle_unbundle_unpacks_objects_into_another_repository() {
    let (tmp, source) = make_repo_with_commit().await;
    let bundle = tmp.path().join("all.bundle");
    let mut create = BundleCommand::create(&bundle);
    create.current_dir(source.path()).all().quiet();
    create.execute().await.unwrap();

    let mut head = RevParseCommand::new();
    head.current_dir(source.path()).arg_str("HEAD");
    let sha = head.execute().await.unwrap();

    let (_target_tmp, target) = common::init_repo().await;
    let mut unbundle = BundleCommand::unbundle(&bundle);
    unbundle.current_dir(target.path());
    let out = unbundle.execute().await.unwrap();

    // unbundle unpacks the objects and reports the refs the bundle carries; it
    // does not update the receiving repository's refs itself.
    assert!(
        out.stdout_str().contains("refs/heads/main"),
        "unexpected unbundle output: {}",
        out.stdout_str()
    );
    let mut kind = CatFileCommand::object_type(&sha);
    kind.current_dir(target.path());
    assert_eq!(kind.execute().await.unwrap(), "commit");
}

#[tokio::test]
async fn bundle_verify_rejects_a_file_that_is_not_a_bundle() {
    let (tmp, repo) = make_repo_with_commit().await;
    let bogus = tmp.path().join("not.bundle");
    std::fs::write(&bogus, "not a bundle\n").unwrap();

    let mut verify = BundleCommand::verify(&bogus);
    verify.current_dir(repo.path());
    match verify.execute().await {
        Err(Error::CommandFailed { .. }) => {}
        other => panic!("expected CommandFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn bundle_create_without_revisions_is_rejected_before_spawning() {
    let (tmp, repo) = make_repo_with_commit().await;
    let bundle = tmp.path().join("empty.bundle");

    let mut create = BundleCommand::create(&bundle);
    create.current_dir(repo.path());
    match create.execute().await {
        Err(Error::InvalidConfig { .. }) => {}
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
    assert!(!bundle.exists(), "git was spawned despite the guard");
}

#[tokio::test]
async fn rerere_clear_drops_the_state_of_the_merge_in_progress() {
    let (_tmp, repo) = make_rerere_conflict().await;
    let merge_rr = repo.path().join(".git").join("MERGE_RR");
    assert!(merge_rr.exists(), "the conflicted merge left no MERGE_RR");

    wait_for_merge_rr_unlocked(&repo).await;

    let mut clear = RerereCommand::clear();
    clear.current_dir(repo.path());
    clear.execute().await.unwrap();

    assert!(!merge_rr.exists(), "clear left MERGE_RR behind");
    let mut status = RerereCommand::status();
    status.current_dir(repo.path());
    let out = status.execute().await.unwrap();
    assert!(
        out.stdout_str().trim().is_empty(),
        "status still reports a path after clear: {}",
        out.stdout_str()
    );
}

#[tokio::test]
async fn rerere_forget_drops_the_recorded_resolution() {
    let (_tmp, repo) = make_rerere_conflict().await;
    record_rerere_resolution(&repo).await;
    assert_eq!(
        rr_cache_entries_with(&repo, "postimage"),
        1,
        "the resolution was not recorded"
    );

    wait_for_merge_rr_unlocked(&repo).await;

    let mut forget = RerereCommand::forget("README");
    forget.current_dir(repo.path());
    forget.execute().await.unwrap();

    assert_eq!(
        rr_cache_entries_with(&repo, "postimage"),
        0,
        "forget left the postimage in place"
    );
}

#[tokio::test]
async fn rerere_gc_keeps_a_freshly_recorded_resolution() {
    let (_tmp, repo) = make_rerere_conflict().await;
    record_rerere_resolution(&repo).await;

    wait_for_merge_rr_unlocked(&repo).await;

    let mut gc = RerereCommand::gc();
    gc.current_dir(repo.path());
    gc.execute().await.unwrap();

    // Pruning is age-based (`gc.rerereResolved` defaults to 60 days), so a
    // resolution recorded moments ago survives.
    assert_eq!(
        rr_cache_entries_with(&repo, "postimage"),
        1,
        "gc pruned a resolution recorded moments ago"
    );
}

#[tokio::test]
async fn sparse_checkout_set_replaces_the_pattern_set() {
    let (_tmp, repo) = make_repo_with_two_directories().await;
    let mut first = SparseCheckoutCommand::set("a");
    first.current_dir(repo.path());
    first.execute().await.unwrap();

    // `set` is not additive: naming only `b` drops `a` again.
    let mut second = SparseCheckoutCommand::set("b");
    second.current_dir(repo.path());
    second.execute().await.unwrap();

    assert_eq!(sparse_patterns(&repo).await, vec!["b"]);
    assert!(!repo.path().join("a").join("f").exists(), "a/f was kept");
}

#[tokio::test]
async fn sparse_checkout_init_no_cone_writes_the_full_pattern_defaults() {
    let (_tmp, repo) = make_repo_with_two_directories().await;

    let mut init = SparseCheckoutCommand::init();
    init.no_cone();
    init.current_dir(repo.path());
    init.execute().await.unwrap();

    // Outside cone mode the patterns are gitignore-style rather than paths:
    // take everything at the root, then exclude every directory.
    assert_eq!(sparse_patterns(&repo).await, vec!["/*", "!/*/"]);
    assert!(repo.path().join("top.txt").exists(), "top.txt was dropped");
    assert!(!repo.path().join("a").join("f").exists(), "a/f was kept");
}

#[tokio::test]
async fn sparse_checkout_disable_restores_the_full_checkout() {
    let (_tmp, repo) = make_repo_with_two_directories().await;
    let mut set = SparseCheckoutCommand::set("a");
    set.current_dir(repo.path());
    set.execute().await.unwrap();
    assert!(!repo.path().join("b").join("f").exists());

    let mut disable = SparseCheckoutCommand::disable();
    disable.current_dir(repo.path());
    disable.execute().await.unwrap();

    assert!(
        repo.path().join("b").join("f").exists(),
        "b/f was not restored"
    );
    // With sparse checkout off there is no pattern set to report, and `list`
    // fails rather than printing nothing.
    let mut list = SparseCheckoutCommand::list();
    list.current_dir(repo.path());
    assert!(
        list.execute().await.is_err(),
        "list succeeded after disable"
    );
}

#[cfg(feature = "parse")]
mod blame_parser {
    use super::*;

    #[tokio::test]
    async fn porcelain_entries_group_lines_by_commit() {
        let (_tmp, repo) = make_repo_with_two_line_history().await;
        let mut cmd = BlameCommand::new();
        cmd.current_dir(repo.path()).file("notes.txt").porcelain();

        let entries = cmd.parse_entries(&cmd.execute().await.unwrap());
        assert_eq!(entries.len(), 3, "unexpected entries: {entries:?}");

        assert_eq!(entries[0].final_line, 1);
        assert_eq!(entries[0].original_line, 1);
        assert_eq!(entries[0].content.as_deref(), Some("one"));
        assert_eq!(entries[0].author.as_deref(), Some("Test"));
        assert_eq!(entries[0].author_mail.as_deref(), Some("test@example.com"));
        assert_eq!(entries[0].summary.as_deref(), Some("add one and two"));
        assert_eq!(entries[0].filename.as_deref(), Some("notes.txt"));
        assert!(entries[0].author_time.is_some());

        // Lines 1 and 2 come from one commit, so the porcelain format writes
        // the group count and the author only on the first of them.
        assert_eq!(entries[1].final_line, 2);
        assert_eq!(entries[1].content.as_deref(), Some("two"));
        assert_eq!(entries[1].sha, entries[0].sha);
        assert_eq!(entries[1].line_count, None);
        assert_eq!(entries[1].author.as_deref(), Some("Test"));
        assert_eq!(entries[1].summary.as_deref(), Some("add one and two"));

        assert_eq!(entries[2].final_line, 3);
        assert_eq!(entries[2].original_line, 3);
        assert_eq!(entries[2].content.as_deref(), Some("three"));
        assert_ne!(entries[2].sha, entries[0].sha);
        assert_eq!(entries[2].summary.as_deref(), Some("add three"));

        // The root commit has no parent to keep walking into, so git marks it
        // as a boundary. Line 2 inherits the flag with the rest of its
        // commit's metadata; the later commit is not a boundary.
        assert!(entries[0].boundary);
        assert!(entries[1].boundary);
        assert!(!entries[2].boundary);
    }

    #[tokio::test]
    async fn line_porcelain_repeats_the_metadata_on_every_line() {
        let (_tmp, repo) = make_repo_with_two_line_history().await;
        let mut cmd = BlameCommand::new();
        cmd.current_dir(repo.path())
            .file("notes.txt")
            .line_porcelain();

        let entries = cmd.parse_entries(&cmd.execute().await.unwrap());
        assert_eq!(entries.len(), 3, "unexpected entries: {entries:?}");
        assert!(
            entries
                .iter()
                .all(|e| e.author.as_deref() == Some("Test") && e.summary.is_some()),
            "metadata missing from an entry: {entries:?}"
        );
    }

    #[tokio::test]
    async fn a_line_range_blames_only_the_requested_lines() {
        let (_tmp, repo) = make_repo_with_two_line_history().await;
        let mut cmd = BlameCommand::new();
        cmd.current_dir(repo.path())
            .file("notes.txt")
            .lines(2, 3)
            .porcelain();

        let entries = cmd.parse_entries(&cmd.execute().await.unwrap());
        assert_eq!(entries.len(), 2, "unexpected entries: {entries:?}");
        assert_eq!(entries[0].final_line, 2);
        assert_eq!(entries[0].content.as_deref(), Some("two"));
        assert_eq!(entries[1].final_line, 3);
        assert_eq!(entries[1].content.as_deref(), Some("three"));
    }
}

/// Build two versions of the same one-commit series: `v1` adds `feature.txt`,
/// and `v2` is that commit amended with one line of the patch rewritten. The
/// two patches stay close enough that range-diff pairs them rather than
/// reporting a drop and an add.
async fn make_two_patch_series(repo: &Repository) {
    repo.checkout().create("v1").execute().await.unwrap();
    std::fs::write(
        repo.path().join("feature.txt"),
        "one\ntwo\nthree\nfour\nfive\n",
    )
    .unwrap();
    repo.add().path("feature.txt").execute().await.unwrap();
    repo.commit()
        .message("add feature")
        .execute()
        .await
        .unwrap();

    repo.checkout()
        .create("v2")
        .target("v1")
        .execute()
        .await
        .unwrap();
    std::fs::write(
        repo.path().join("feature.txt"),
        "one\ntwo\nthree\nfour\nFIVE\n",
    )
    .unwrap();
    repo.add().path("feature.txt").execute().await.unwrap();
    repo.commit().amend().no_edit().execute().await.unwrap();
}

#[tokio::test]
async fn range_diff_pairs_a_rewritten_commit() {
    let (_tmp, repo) = make_repo_with_commit().await;
    make_two_patch_series(&repo).await;

    let mut cmd = RangeDiffCommand::new();
    cmd.current_dir(repo.path()).rev("main..v1").rev("main..v2");
    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(
        stdout.contains("add feature"),
        "expected the commit subject: {stdout}"
    );
    assert!(
        stdout.contains('!'),
        "expected a changed-commit marker: {stdout}"
    );
}

#[tokio::test]
async fn range_diff_accepts_the_symmetric_form() {
    let (_tmp, repo) = make_repo_with_commit().await;
    make_two_patch_series(&repo).await;

    let mut cmd = RangeDiffCommand::new();
    cmd.current_dir(repo.path())
        .rev("v1...v2")
        .creation_factor(90);
    let out = cmd.execute().await.unwrap();
    assert!(
        out.stdout_str().contains("add feature"),
        "expected the commit subject: {}",
        out.stdout_str()
    );
}

#[tokio::test]
async fn range_diff_without_revisions_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = RangeDiffCommand::new();
    cmd.current_dir(repo.path());
    assert!(cmd.execute().await.is_err());
}

#[tokio::test]
async fn range_diff_with_four_revisions_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = RangeDiffCommand::new();
    cmd.current_dir(repo.path())
        .rev("main")
        .rev("v1")
        .rev("v2")
        .rev("v3");
    assert!(cmd.execute().await.is_err());
}

/// Resolve a revision to its full SHA.
async fn rev(repo: &Repository, revision: &str) -> String {
    let mut cmd = RevParseCommand::new();
    cmd.current_dir(repo.path()).arg_str(revision);
    cmd.execute().await.unwrap()
}

/// A repository whose `main` and `feature` branches diverge after the initial
/// commit, one commit each. Returns the SHA they forked from.
async fn make_diverged_branches(repo: &Repository) -> String {
    let base = rev(repo, "HEAD").await;
    repo.checkout().create("feature").execute().await.unwrap();
    commit_file(repo, "feature.txt", "feature\n", "add feature").await;
    repo.checkout().target("main").execute().await.unwrap();
    commit_file(repo, "main.txt", "main\n", "add main").await;
    base
}

#[tokio::test]
async fn merge_base_finds_the_fork_commit() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let base = make_diverged_branches(&repo).await;

    let mut cmd = MergeBaseCommand::new();
    cmd.current_dir(repo.path())
        .commit("main")
        .commit("feature");
    let out = cmd.execute().await.unwrap();
    assert_eq!(out.stdout_str().trim(), base);
}

#[tokio::test]
async fn merge_base_is_ancestor_answers_both_ways() {
    let (_tmp, repo) = make_repo_with_commit().await;
    make_diverged_branches(&repo).await;

    let mut yes = MergeBaseCommand::new();
    yes.current_dir(repo.path())
        .is_ancestor()
        .commit("main~1")
        .commit("feature");
    assert!(yes.execute_is_ancestor().await.unwrap());

    let mut no = MergeBaseCommand::new();
    no.current_dir(repo.path())
        .is_ancestor()
        .commit("main")
        .commit("feature");
    assert!(!no.execute_is_ancestor().await.unwrap());
}

#[tokio::test]
async fn merge_base_is_ancestor_needs_the_flag_and_two_commits() {
    let (_tmp, repo) = make_repo_with_commit().await;

    // The flag decides both the output and what exit 1 means.
    let mut unflagged = MergeBaseCommand::new();
    unflagged
        .current_dir(repo.path())
        .commit("main")
        .commit("main");
    assert!(unflagged.execute_is_ancestor().await.is_err());

    let mut three = MergeBaseCommand::new();
    three
        .current_dir(repo.path())
        .is_ancestor()
        .commits(["main", "main", "main"]);
    assert!(three.execute_is_ancestor().await.is_err());
    assert!(three.execute().await.is_err());
}

#[tokio::test]
async fn merge_base_unrelated_histories_have_no_base() {
    let (_tmp, repo) = make_repo_with_commit().await;
    repo.checkout().orphan("detached").execute().await.unwrap();
    commit_file(&repo, "other.txt", "other\n", "unrelated root").await;

    let mut cmd = MergeBaseCommand::new();
    cmd.current_dir(repo.path())
        .commit("main")
        .commit("detached");
    // git exits 1 with no output, which execute() cannot tell from a failure.
    assert!(cmd.execute().await.is_err());
    assert!(cmd.execute_allow_no_base().await.unwrap().is_none());
}

#[tokio::test]
async fn merge_base_all_reports_a_base_that_allow_no_base_unwraps() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let base = make_diverged_branches(&repo).await;

    let mut cmd = MergeBaseCommand::new();
    cmd.current_dir(repo.path())
        .all()
        .commit("main")
        .commit("feature");
    let out = cmd.execute_allow_no_base().await.unwrap().unwrap();
    assert_eq!(out.stdout_str().trim(), base);
}

#[tokio::test]
async fn merge_base_fork_point_reads_the_reflog() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let base = make_diverged_branches(&repo).await;

    let mut cmd = MergeBaseCommand::new();
    cmd.current_dir(repo.path())
        .fork_point()
        .commit("main")
        .commit("feature");
    let out = cmd.execute().await.unwrap();
    assert_eq!(out.stdout_str().trim(), base);
}

#[tokio::test]
async fn merge_base_rejects_bad_argument_counts_and_flag_combinations() {
    let (_tmp, repo) = make_repo_with_commit().await;

    let mut one = MergeBaseCommand::new();
    one.current_dir(repo.path()).commit("main");
    assert!(matches!(
        one.execute().await,
        Err(Error::InvalidConfig { .. })
    ));

    let mut three_fork_points = MergeBaseCommand::new();
    three_fork_points
        .current_dir(repo.path())
        .fork_point()
        .commits(["main", "feature", "topic"]);
    assert!(three_fork_points.execute().await.is_err());

    let mut ancestor_and_all = MergeBaseCommand::new();
    ancestor_and_all
        .current_dir(repo.path())
        .is_ancestor()
        .all()
        .commit("main")
        .commit("main");
    assert!(ancestor_and_all.execute().await.is_err());

    let mut ancestor_and_fork_point = MergeBaseCommand::new();
    ancestor_and_fork_point
        .current_dir(repo.path())
        .is_ancestor()
        .fork_point()
        .commit("main")
        .commit("main");
    assert!(ancestor_and_fork_point.execute().await.is_err());
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

/// Whether `haystack` contains `needle` as a byte substring. Archive entry
/// names sit in NUL-padded header fields, so a plain substring search is the
/// way to look for one without decoding the container.
fn contains_bytes(haystack: &[u8], needle: &str) -> bool {
    let needle = needle.as_bytes();
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[tokio::test]
async fn archive_writes_a_tar_to_stdout() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = ArchiveCommand::new("HEAD");
    cmd.current_dir(repo.path()).format(ArchiveFormat::Tar);
    let out = cmd.execute().await.unwrap();

    let bytes = out.stdout_bytes();
    assert!(!bytes.is_empty(), "archive should not be empty");
    // Archiving a commit puts a pax_global_header entry first, so the file's
    // own header is not at offset 0.
    assert!(
        contains_bytes(bytes, "hello.txt"),
        "tar should name the committed file"
    );
}

#[tokio::test]
async fn archive_output_file_carries_the_prefix() {
    let (tmp, repo) = make_repo_with_commit().await;
    let dest = tmp.path().join("out.tar");

    let mut cmd = ArchiveCommand::new("HEAD");
    cmd.current_dir(repo.path())
        .format(ArchiveFormat::Tar)
        .prefix("proj/")
        .output(&dest);
    cmd.execute().await.unwrap();

    let bytes = std::fs::read(&dest).unwrap();
    assert!(
        contains_bytes(&bytes, "proj/hello.txt"),
        "--prefix should be prepended to the archived path"
    );
}

#[tokio::test]
async fn clean_dry_run_reports_without_removing() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let untracked = repo.path().join("scratch.txt");
    std::fs::write(&untracked, "scratch\n").unwrap();

    let mut cmd = CleanCommand::new();
    cmd.current_dir(repo.path()).dry_run();
    let out = cmd.execute().await.unwrap();

    assert!(
        out.stdout_str().contains("scratch.txt"),
        "dry run did not report the file: {}",
        out.stdout_str()
    );
    assert!(untracked.exists(), "dry run removed the file");
}

#[tokio::test]
async fn clean_force_removes_untracked_files() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let untracked = repo.path().join("scratch.txt");
    std::fs::write(&untracked, "scratch\n").unwrap();

    let mut cmd = CleanCommand::new();
    cmd.current_dir(repo.path()).force();
    cmd.execute().await.unwrap();

    assert!(!untracked.exists(), "the untracked file survived");
    assert!(
        repo.path().join("hello.txt").exists(),
        "a tracked file was removed"
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

#[tokio::test]
async fn clean_needs_directories_to_recurse() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let dir = repo.path().join("scratch");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join("inner.txt"), "inner\n").unwrap();

    let mut cmd = CleanCommand::new();
    cmd.current_dir(repo.path()).force();
    cmd.execute().await.unwrap();
    assert!(dir.exists(), "the directory went without -d");

    let mut cmd = CleanCommand::new();
    cmd.current_dir(repo.path()).force().directories();
    cmd.execute().await.unwrap();
    assert!(!dir.exists(), "-d did not remove the directory");
}

#[tokio::test]
async fn clean_needs_ignored_to_remove_ignored_files() {
    let (_tmp, repo) = make_repo_with_commit().await;
    std::fs::write(repo.path().join(".gitignore"), "ignored.txt\n").unwrap();
    repo.add().path(".gitignore").execute().await.unwrap();
    repo.commit().message("ignore").execute().await.unwrap();

    let ignored = repo.path().join("ignored.txt");
    std::fs::write(&ignored, "ignored\n").unwrap();

    let mut cmd = CleanCommand::new();
    cmd.current_dir(repo.path()).force();
    cmd.execute().await.unwrap();
    assert!(ignored.exists(), "the ignored file went without -x");

    let mut cmd = CleanCommand::new();
    cmd.current_dir(repo.path()).force().ignored();
    cmd.execute().await.unwrap();
    assert!(!ignored.exists(), "-x did not remove the ignored file");
}

#[tokio::test]
async fn clean_pathspecs_limit_what_is_removed() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let kept = repo.path().join("keep.txt");
    let removed = repo.path().join("drop.txt");
    std::fs::write(&kept, "keep\n").unwrap();
    std::fs::write(&removed, "drop\n").unwrap();

    let mut cmd = CleanCommand::new();
    cmd.current_dir(repo.path()).force().path("drop.txt");
    cmd.execute().await.unwrap();

    assert!(!removed.exists(), "the matching path survived");
    assert!(kept.exists(), "a path outside the pathspec was removed");
}

#[tokio::test]
async fn revert_records_a_reversing_commit() {
    let (_tmp, repo) = make_repo_with_commit().await;
    commit_file(&repo, "feature.txt", "feature\n", "add feature").await;
    assert!(repo.path().join("feature.txt").exists());

    let mut cmd = RevertCommand::new();
    cmd.current_dir(repo.path()).commit("HEAD").no_edit();
    cmd.execute().await.unwrap();

    assert!(
        !repo.path().join("feature.txt").exists(),
        "revert did not undo the commit"
    );

    let mut log = LogCommand::new();
    log.current_dir(repo.path()).max_count(1).format("%s");
    let subject = log.execute().await.unwrap().stdout_str().to_string();
    assert!(
        subject.contains(r#"Revert "add feature""#),
        "unexpected subject: {subject}"
    );
}

#[tokio::test]
async fn revert_no_commit_leaves_head_alone() {
    let (_tmp, repo) = make_repo_with_commit().await;
    commit_file(&repo, "feature.txt", "feature\n", "add feature").await;

    let mut head = RevParseCommand::new();
    head.current_dir(repo.path()).arg_str("HEAD");
    let before = head.execute().await.unwrap();

    let mut cmd = RevertCommand::new();
    cmd.current_dir(repo.path()).commit("HEAD").no_commit();
    cmd.execute().await.unwrap();

    let after = head.execute().await.unwrap();
    assert_eq!(before, after, "--no-commit moved HEAD");
    assert!(
        !repo.path().join("feature.txt").exists(),
        "--no-commit did not apply the reversal to the working tree"
    );
}

#[tokio::test]
async fn revert_of_a_merge_needs_a_mainline() {
    let (_tmp, repo) = make_repo_with_commit().await;
    repo.checkout().create("feature").execute().await.unwrap();
    commit_file(&repo, "feature.txt", "feature\n", "add feature").await;
    repo.checkout().target("main").execute().await.unwrap();
    commit_file(&repo, "other.txt", "other\n", "add other").await;
    repo.merge()
        .commit_ref("feature")
        .no_ff()
        .message("merge feature")
        .execute()
        .await
        .unwrap();

    // git refuses a merge revert without --mainline.
    let mut cmd = RevertCommand::new();
    cmd.current_dir(repo.path()).commit("HEAD").no_edit();
    assert!(cmd.execute().await.is_err());

    let mut cmd = RevertCommand::new();
    cmd.current_dir(repo.path())
        .commit("HEAD")
        .mainline(1)
        .no_edit();
    cmd.execute().await.unwrap();
    assert!(
        !repo.path().join("feature.txt").exists(),
        "reverting the merge kept the merged-in file"
    );
}

async fn make_repo_with_two_authors() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;
    commit_file(&repo, "one.txt", "one\n", "add one").await;
    commit_file(&repo, "two.txt", "two\n", "add two").await;

    // A third commit from a different author, so the report has two groups.
    std::fs::write(repo.path().join("three.txt"), "three\n").unwrap();
    repo.add().path("three.txt").execute().await.unwrap();
    repo.commit()
        .message("add three")
        .author("Other Dev <other@example.com>")
        .execute()
        .await
        .unwrap();
    (tmp, repo)
}

/// A local repository is a valid `ls-remote` target, so these tests query a
/// real remote without touching the network.
async fn make_remote_with_a_tag() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = make_repo_with_commit().await;
    repo.tag()
        .name("v1.0")
        .annotated()
        .message("release 1.0")
        .execute()
        .await
        .unwrap();
    (tmp, repo)
}

#[tokio::test]
async fn shortlog_groups_commits_by_author() {
    let (_tmp, repo) = make_repo_with_two_authors().await;
    let mut cmd = ShortlogCommand::new();
    cmd.current_dir(repo.path()).rev("HEAD");

    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(stdout.contains("Test (2):"), "unexpected report: {stdout}");
    assert!(
        stdout.contains("Other Dev (1):"),
        "unexpected report: {stdout}"
    );
    assert!(stdout.contains("add one"), "unexpected report: {stdout}");
}

#[tokio::test]
async fn shortlog_summary_drops_the_subjects() {
    let (_tmp, repo) = make_repo_with_two_authors().await;
    let mut cmd = ShortlogCommand::new();
    cmd.current_dir(repo.path())
        .rev("HEAD")
        .summary()
        .numbered();

    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(stdout.contains("Test"), "unexpected report: {stdout}");
    assert!(
        !stdout.contains("add one"),
        "subjects survived --summary: {stdout}"
    );
    // --numbered puts the two-commit author first.
    let first = stdout.lines().next().unwrap_or_default();
    assert!(first.contains("Test"), "unexpected first line: {stdout}");
}

#[tokio::test]
async fn shortlog_pathspecs_limit_the_report() {
    let (_tmp, repo) = make_repo_with_two_authors().await;
    let mut cmd = ShortlogCommand::new();
    cmd.current_dir(repo.path()).rev("HEAD").path("three.txt");

    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();
    assert!(stdout.contains("add three"), "unexpected report: {stdout}");
    assert!(
        !stdout.contains("add one"),
        "the pathspec did not limit the report: {stdout}"
    );
}

#[tokio::test]
async fn ls_remote_lists_a_local_repositorys_refs() {
    let (_tmp, remote) = make_remote_with_a_tag().await;

    let cmd = LsRemoteCommand::remote(remote.path().to_string_lossy());
    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();

    assert!(
        stdout.contains("refs/heads/main"),
        "missing branch: {stdout}"
    );
    assert!(stdout.contains("refs/tags/v1.0"), "missing tag: {stdout}");
    assert!(stdout.contains("HEAD"), "missing HEAD: {stdout}");
}

#[tokio::test]
async fn ls_remote_heads_excludes_tags() {
    let (_tmp, remote) = make_remote_with_a_tag().await;

    let mut cmd = LsRemoteCommand::remote(remote.path().to_string_lossy());
    cmd.heads();
    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();

    assert!(
        stdout.contains("refs/heads/main"),
        "missing branch: {stdout}"
    );
    assert!(
        !stdout.contains("refs/tags/"),
        "tags were not filtered: {stdout}"
    );
}

#[tokio::test]
async fn ls_remote_pattern_narrows_the_output() {
    let (_tmp, remote) = make_remote_with_a_tag().await;

    let mut cmd = LsRemoteCommand::remote(remote.path().to_string_lossy());
    cmd.pattern("refs/tags/*");
    let out = cmd.execute().await.unwrap();
    let stdout = out.stdout_str();

    assert!(stdout.contains("refs/tags/v1.0"), "missing tag: {stdout}");
    assert!(
        !stdout.contains("refs/heads/"),
        "branches were not filtered: {stdout}"
    );
}

#[tokio::test]
async fn archive_zip_writes_a_zip_file() {
    let (tmp, repo) = make_repo_with_commit().await;
    let dest = tmp.path().join("out.zip");

    let mut cmd = ArchiveCommand::new("HEAD");
    cmd.current_dir(repo.path())
        .format(ArchiveFormat::Zip)
        .output(&dest);
    cmd.execute().await.unwrap();

    let bytes = std::fs::read(&dest).unwrap();
    assert_eq!(
        &bytes[..4],
        b"PK\x03\x04",
        "expected a zip local file header"
    );
    assert!(contains_bytes(&bytes, "hello.txt"));
}

#[tokio::test]
async fn archive_path_limits_the_contents() {
    let (_tmp, repo) = make_repo_with_commit().await;
    std::fs::write(repo.path().join("other.txt"), "other\n").unwrap();
    repo.add().path("other.txt").execute().await.unwrap();
    repo.commit().message("second").execute().await.unwrap();

    let mut cmd = ArchiveCommand::new("HEAD");
    cmd.current_dir(repo.path())
        .format(ArchiveFormat::Tar)
        .path("hello.txt");
    let out = cmd.execute().await.unwrap();

    let bytes = out.stdout_bytes();
    assert!(contains_bytes(bytes, "hello.txt"));
    assert!(
        !contains_bytes(bytes, "other.txt"),
        "a pathspec should exclude the other file"
    );
}

/// Run `name-rev --name-only` over `revs` and return one name per line.
async fn names_of(repo: &Repository, revs: &[&str]) -> Vec<String> {
    let mut cmd = NameRevCommand::new();
    cmd.current_dir(repo.path()).name_only().revs(revs.to_vec());
    let out = cmd.execute().await.unwrap();
    out.stdout_str().lines().map(str::to_string).collect()
}

#[tokio::test]
async fn name_rev_names_head_after_its_branch() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let sha = rev(&repo, "HEAD").await;

    let mut cmd = NameRevCommand::new();
    cmd.current_dir(repo.path()).rev(&sha);
    let out = cmd.execute().await.unwrap();
    // The default form is "<rev> <name>", the name carrying the distance back
    // from the ref's tip.
    assert_eq!(out.stdout_str().trim(), format!("{sha} main"));
}

#[tokio::test]
async fn name_rev_name_only_drops_the_revision_column() {
    let (_tmp, repo) = make_repo_with_commit().await;
    assert_eq!(names_of(&repo, &["HEAD"]).await, vec!["main"]);
}

#[tokio::test]
async fn name_rev_names_several_revisions_in_order() {
    let (_tmp, repo) = make_repo_with_commit().await;
    commit_file(&repo, "second.txt", "second\n", "add second").await;

    assert_eq!(
        names_of(&repo, &["HEAD", "HEAD~1"]).await,
        vec!["main", "main~1"]
    );
}

#[tokio::test]
async fn name_rev_tags_considers_only_tags() {
    let (_tmp, repo) = make_repo_with_commit().await;
    repo.tag().name("v0.1.0").execute().await.unwrap();
    commit_file(&repo, "second.txt", "second\n", "add second").await;

    // Every ref is a candidate by default, and only the branch reaches HEAD.
    assert_eq!(
        names_of(&repo, &["HEAD", "HEAD~1"]).await,
        vec!["main", "tags/v0.1.0"]
    );

    let mut tagged = NameRevCommand::new();
    tagged
        .current_dir(repo.path())
        .name_only()
        .tags()
        .rev("HEAD")
        .rev("HEAD~1");
    let out = tagged.execute().await.unwrap();
    // Restricted to tags, the commit past the tag has no name left, and the
    // tagged one loses the `tags/` prefix it carries when every ref competes.
    assert_eq!(
        out.stdout_str().lines().collect::<Vec<_>>(),
        vec!["undefined", "v0.1.0"]
    );
}

#[tokio::test]
async fn name_rev_refs_restricts_the_candidate_refs() {
    let (_tmp, repo) = make_repo_with_commit().await;
    repo.tag().name("v0.1.0").execute().await.unwrap();

    let mut only_tags = NameRevCommand::new();
    only_tags
        .current_dir(repo.path())
        .name_only()
        .refs("refs/tags/*")
        .rev("HEAD");
    assert_eq!(
        only_tags.execute().await.unwrap().stdout_str().trim(),
        "tags/v0.1.0"
    );

    let mut only_heads = NameRevCommand::new();
    only_heads
        .current_dir(repo.path())
        .name_only()
        .refs("refs/heads/*")
        .rev("HEAD");
    assert_eq!(
        only_heads.execute().await.unwrap().stdout_str().trim(),
        "main"
    );
}

#[tokio::test]
async fn name_rev_reports_undefined_when_no_candidate_ref_reaches_the_revision() {
    let (_tmp, repo) = make_repo_with_commit().await;

    let mut cmd = NameRevCommand::new();
    cmd.current_dir(repo.path())
        .name_only()
        .refs("refs/tags/nothing-matches-this/*")
        .rev("HEAD");
    // Naming nothing is not a failure: git exits 0 and says so in the output.
    assert_eq!(
        cmd.execute().await.unwrap().stdout_str().trim(),
        "undefined"
    );
}

#[tokio::test]
async fn revert_without_a_commit_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = RevertCommand::new();
    cmd.current_dir(repo.path()).no_edit();
    assert!(matches!(
        cmd.execute().await,
        Err(Error::InvalidConfig { .. })
    ));
}

#[tokio::test]
async fn shortlog_without_a_revision_is_rejected() {
    let (_tmp, repo) = make_repo_with_two_authors().await;
    let mut cmd = ShortlogCommand::new();
    cmd.current_dir(repo.path());
    assert!(matches!(
        cmd.execute().await,
        Err(Error::InvalidConfig { .. })
    ));
}

#[tokio::test]
async fn revert_rejects_two_session_actions() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = RevertCommand::new();
    cmd.current_dir(repo.path()).abort().skip();
    assert!(matches!(
        cmd.execute().await,
        Err(Error::InvalidConfig { .. })
    ));
}

#[cfg(feature = "parse")]
mod shortlog_parser {
    use super::*;

    #[tokio::test]
    async fn entries_carry_each_author_and_their_subjects() {
        let (_tmp, repo) = make_repo_with_two_authors().await;
        let mut cmd = ShortlogCommand::new();
        cmd.current_dir(repo.path()).rev("HEAD").numbered();

        let out = cmd.execute().await.unwrap();
        let entries = cmd.parse_entries(&out);
        assert_eq!(entries.len(), 2, "unexpected entries: {entries:?}");

        assert_eq!(entries[0].author, "Test");
        assert_eq!(entries[0].email, None);
        assert_eq!(entries[0].count, 2);
        assert_eq!(entries[0].subjects, ["add one", "add two"]);

        assert_eq!(entries[1].author, "Other Dev");
        assert_eq!(entries[1].count, 1);
        assert_eq!(entries[1].subjects, ["add three"]);
    }

    #[tokio::test]
    async fn summary_entries_carry_counts_and_emails_without_subjects() {
        let (_tmp, repo) = make_repo_with_two_authors().await;
        let mut cmd = ShortlogCommand::new();
        cmd.current_dir(repo.path())
            .rev("HEAD")
            .summary()
            .numbered()
            .email();

        let out = cmd.execute().await.unwrap();
        let entries = cmd.parse_entries(&out);
        assert_eq!(entries.len(), 2, "unexpected entries: {entries:?}");

        assert_eq!(entries[0].author, "Test");
        assert_eq!(entries[0].email.as_deref(), Some("test@example.com"));
        assert_eq!(entries[0].count, 2);
        assert!(entries[0].subjects.is_empty());

        assert_eq!(entries[1].author, "Other Dev");
        assert_eq!(entries[1].email.as_deref(), Some("other@example.com"));
        assert_eq!(entries[1].count, 1);
    }

    #[tokio::test]
    async fn grouping_by_committer_credits_the_committing_identity() {
        let (_tmp, repo) = make_repo_with_two_authors().await;
        let mut cmd = ShortlogCommand::new();
        cmd.current_dir(repo.path())
            .rev("HEAD")
            .summary()
            .committer();

        let out = cmd.execute().await.unwrap();
        let entries = cmd.parse_entries(&out);
        // All three commits were made by the configured identity, whatever the
        // --author override said.
        assert_eq!(entries.len(), 1, "unexpected entries: {entries:?}");
        assert_eq!(entries[0].author, "Test");
        assert_eq!(entries[0].count, 3);
    }
}

#[tokio::test]
async fn gc_packs_loose_objects() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = GcCommand::new();
    cmd.current_dir(repo.path());
    cmd.execute().await.unwrap();

    // gc repacks reachable objects, so a pack file must now exist.
    let pack_dir = repo.path().join(".git/objects/pack");
    let has_pack = std::fs::read_dir(&pack_dir)
        .unwrap()
        .filter_map(Result::ok)
        .any(|e| e.file_name().to_string_lossy().ends_with(".pack"));
    assert!(
        has_pack,
        "expected a pack file under {}",
        pack_dir.display()
    );
}

#[tokio::test]
async fn gc_prune_now_drops_an_unreachable_object() {
    let (_tmp, repo) = make_repo_with_commit().await;

    // Write a loose blob that nothing references.
    let blob = repo.path().join("dangling.txt");
    std::fs::write(&blob, "dangling\n").unwrap();
    let mut h = HashObjectCommand::new();
    h.current_dir(repo.path()).write().path(&blob);
    let sha = h.execute().await.unwrap();

    let obj = repo
        .path()
        .join(".git/objects")
        .join(&sha[..2])
        .join(&sha[2..]);
    assert!(obj.exists(), "loose object should exist before prune");

    let mut cmd = GcCommand::new();
    cmd.current_dir(repo.path()).prune("now");
    cmd.execute().await.unwrap();

    assert!(!obj.exists(), "unreachable object should be pruned");
}

#[tokio::test]
async fn fsck_reports_nothing_for_a_clean_repo() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = FsckCommand::new();
    cmd.current_dir(repo.path());
    let out = cmd.execute().await.unwrap();
    assert_eq!(out.stdout_str().trim(), "");
}

/// Writes a blob that nothing references and returns its SHA.
async fn write_unreferenced_blob(repo: &Repository) -> String {
    let blob = repo.path().join("loose.txt");
    std::fs::write(&blob, "loose\n").unwrap();
    let mut h = HashObjectCommand::new();
    h.current_dir(repo.path()).write().path(&blob);
    h.execute().await.unwrap()
}

#[tokio::test]
async fn fsck_reports_a_dangling_object_by_default() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let sha = write_unreferenced_blob(&repo).await;

    let mut cmd = FsckCommand::new();
    cmd.current_dir(repo.path());
    let out = cmd.execute().await.unwrap();
    assert!(
        out.stdout_str().contains(&format!("dangling blob {sha}")),
        "unexpected report: {}",
        out.stdout_str()
    );
}

// `git maintenance`. Registration writes `maintenance.repo` to the user's
// global config, so every test here passes `--config-file` pointing into the
// tempdir: the real global config is never touched.

#[tokio::test]
async fn maintenance_run_writes_the_commit_graph() {
    let (_tmp, repo) = make_repo_with_commit().await;
    // The commit-graph task writes a split chain under commit-graphs/, not the
    // single commit-graph file that a full `git gc` produces. Verified against
    // git 2.50.1 before the assertion was written.
    let chain = repo
        .path()
        .join(".git/objects/info/commit-graphs/commit-graph-chain");
    assert!(
        !chain.exists(),
        "commit graph exists before maintenance ran"
    );

    let mut cmd = MaintenanceCommand::run();
    cmd.current_dir(repo.path())
        .quiet()
        .task(MaintenanceTask::CommitGraph);
    cmd.execute().await.unwrap();

    assert!(chain.exists(), "maintenance run did not write {chain:?}");
}

#[tokio::test]
async fn maintenance_run_with_an_unknown_task_fails() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = MaintenanceCommand::run();
    cmd.current_dir(repo.path()).task_raw("not-a-task");
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::CommandFailed { .. }),
        "expected a non-zero exit, got {err:?}"
    );
}

#[tokio::test]
async fn fsck_no_dangling_suppresses_the_report() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let sha = write_unreferenced_blob(&repo).await;

    let mut cmd = FsckCommand::new();
    cmd.current_dir(repo.path()).no_dangling();
    let out = cmd.execute().await.unwrap();
    assert!(
        !out.stdout_str().contains(&sha),
        "unexpected report: {}",
        out.stdout_str()
    );
}

#[tokio::test]
async fn maintenance_register_then_unregister_edits_the_given_config_file() {
    let (tmp, repo) = make_repo_with_commit().await;
    let config = tmp.path().join("fake-global-config");

    let mut cmd = MaintenanceCommand::register();
    cmd.current_dir(repo.path()).config_file(&config);
    cmd.execute().await.unwrap();

    let registered = std::fs::read_to_string(&config).unwrap();
    assert!(
        registered.contains("repo = "),
        "no repo entry in {registered:?}"
    );

    let mut cmd = MaintenanceCommand::unregister();
    cmd.current_dir(repo.path()).config_file(&config);
    cmd.execute().await.unwrap();

    let unregistered = std::fs::read_to_string(&config).unwrap();
    assert!(
        !unregistered.contains("repo = "),
        "repo entry survived unregister: {unregistered:?}"
    );
}

#[tokio::test]
async fn fsck_full_unreachable_names_the_object() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let sha = write_unreferenced_blob(&repo).await;

    let mut cmd = FsckCommand::new();
    cmd.current_dir(repo.path()).full().unreachable();
    let out = cmd.execute().await.unwrap();
    // --unreachable reports the same object under the unreachable heading.
    assert!(
        out.stdout_str()
            .contains(&format!("unreachable blob {sha}")),
        "unexpected report: {}",
        out.stdout_str()
    );
}

#[tokio::test]
async fn maintenance_unregister_needs_force_when_not_registered() {
    let (tmp, repo) = make_repo_with_commit().await;
    let config = tmp.path().join("fake-global-config");
    std::fs::write(&config, "").unwrap();

    let mut cmd = MaintenanceCommand::unregister();
    cmd.current_dir(repo.path()).config_file(&config);
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::CommandFailed { .. }),
        "expected unregistering an unregistered repo to fail, got {err:?}"
    );

    let mut cmd = MaintenanceCommand::unregister();
    cmd.current_dir(repo.path()).force().config_file(&config);
    cmd.execute().await.unwrap();
}

#[tokio::test]
async fn archive_rejects_an_unknown_format() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = ArchiveCommand::new("HEAD");
    cmd.current_dir(repo.path()).format_raw("bogus");
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::CommandFailed { .. }),
        "unexpected error: {err:?}"
    );
}

#[tokio::test]
async fn ls_remote_patterns_without_a_repository_are_rejected() {
    let mut cmd = LsRemoteCommand::new();
    cmd.pattern("refs/heads/*");
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidConfig { .. }),
        "expected an invalid-config error, got {err:?}"
    );
}

#[tokio::test]
async fn name_rev_without_a_revision_is_rejected() {
    let (_tmp, repo) = make_repo_with_commit().await;
    let mut cmd = NameRevCommand::new();
    cmd.current_dir(repo.path()).tags();
    let err = cmd.execute().await.unwrap_err();
    assert!(
        matches!(err, Error::InvalidConfig { .. }),
        "expected an invalid-config error, got {err:?}"
    );
}

#[cfg(feature = "parse")]
mod ls_remote_parser {
    use super::*;
    use git_spawn::parse::parse_ls_remote_symrefs;

    #[tokio::test]
    async fn an_annotated_tag_yields_a_peeled_entry_for_the_commit() {
        let (_tmp, remote) = make_remote_with_a_tag().await;
        let head = {
            let mut rev = RevParseCommand::new();
            rev.current_dir(remote.path()).arg_str("HEAD");
            rev.execute().await.unwrap()
        };

        let mut cmd = LsRemoteCommand::remote(remote.path().to_string_lossy());
        cmd.tags();
        let entries = cmd.parse_entries(&cmd.execute().await.unwrap());

        let tags: Vec<_> = entries
            .iter()
            .filter(|e| e.name == "refs/tags/v1.0")
            .collect();
        assert_eq!(
            tags.len(),
            2,
            "expected the tag and its peeled line: {entries:?}"
        );

        let unpeeled = tags.iter().find(|e| !e.peeled).unwrap();
        let peeled = tags.iter().find(|e| e.peeled).unwrap();
        assert_eq!(peeled.sha, head, "the peeled line should name the commit");
        assert_ne!(
            unpeeled.sha, peeled.sha,
            "an annotated tag object is distinct from its commit"
        );
    }

    #[tokio::test]
    async fn refs_flag_drops_head_and_the_peeled_lines() {
        let (_tmp, remote) = make_remote_with_a_tag().await;

        let mut cmd = LsRemoteCommand::remote(remote.path().to_string_lossy());
        cmd.refs();
        let entries = cmd.parse_entries(&cmd.execute().await.unwrap());

        assert!(!entries.is_empty(), "expected some refs");
        assert!(
            entries.iter().all(|e| !e.peeled),
            "peeled entries survived: {entries:?}"
        );
        assert!(
            entries.iter().all(|e| e.name != "HEAD"),
            "HEAD survived: {entries:?}"
        );
        assert!(entries.iter().any(|e| e.name == "refs/heads/main"));
    }

    #[tokio::test]
    async fn symref_reports_what_head_points_at() {
        let (_tmp, remote) = make_remote_with_a_tag().await;

        let mut cmd = LsRemoteCommand::remote(remote.path().to_string_lossy());
        cmd.symref();
        let out = cmd.execute().await.unwrap();

        let symrefs = parse_ls_remote_symrefs(&out.stdout_str());
        assert_eq!(
            symrefs
                .iter()
                .find(|(name, _)| name == "HEAD")
                .map(|(_, target)| target.as_str()),
            Some("refs/heads/main"),
            "unexpected symrefs: {symrefs:?}"
        );

        // The symref line is not itself an entry, but HEAD still is.
        let entries = cmd.parse_entries(&out);
        assert!(entries.iter().any(|e| e.name == "HEAD"), "{entries:?}");
        assert!(
            entries.iter().all(|e| !e.sha.starts_with("ref: ")),
            "{entries:?}"
        );
    }
}
