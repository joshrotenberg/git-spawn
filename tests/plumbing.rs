//! Integration tests for plumbing commands and typed parsers.

use git_spawn::{
    CatFileCommand, DescribeCommand, ForEachRefCommand, GitCommand, HashObjectCommand,
    LsFilesCommand, LsTreeCommand, Repository, RevParseCommand, ShowRefCommand, SymbolicRefCommand,
    UpdateRefCommand,
};

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
