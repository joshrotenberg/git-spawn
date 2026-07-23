//! Pure argv assertions: verifies each command builds the right argument vector
//! without spawning `git`.

use git_spawn::command::{
    reset::ResetMode,
    stash::{StashAction, StashCommand},
    status::StatusFormat,
};
use git_spawn::*;

fn args_of<C: GitCommand>(c: &C) -> Vec<String> {
    c.build_command_args()
}

#[test]
fn init_plain() {
    let c = InitCommand::in_directory("/tmp/r");
    assert_eq!(args_of(&c), vec!["init", "/tmp/r"]);
}

#[test]
fn init_bare_with_branch() {
    let mut c = InitCommand::in_directory("/tmp/r");
    c.bare().initial_branch("main").quiet();
    assert_eq!(
        args_of(&c),
        vec![
            "init",
            "--bare",
            "--quiet",
            "--initial-branch=main",
            "/tmp/r"
        ]
    );
}

#[test]
fn clone_basic() {
    let mut c = CloneCommand::new("https://example.com/foo.git");
    c.directory("/tmp/foo").depth(1).single_branch();
    assert_eq!(
        args_of(&c),
        vec![
            "clone",
            "--depth=1",
            "--single-branch",
            "https://example.com/foo.git",
            "/tmp/foo",
        ]
    );
}

#[test]
fn add_paths_with_flags() {
    let mut c = AddCommand::new();
    c.all().verbose().paths(["a.txt", "b.txt"]);
    assert_eq!(
        args_of(&c),
        vec!["add", "--all", "--verbose", "--", "a.txt", "b.txt"]
    );
}

#[test]
fn commit_with_message_and_amend() {
    let mut c = CommitCommand::with_message("hello");
    c.amend().no_edit().signoff();
    let a = args_of(&c);
    assert!(a.contains(&"--amend".to_string()));
    assert!(a.contains(&"--no-edit".to_string()));
    assert!(a.contains(&"--signoff".to_string()));
    assert!(a.contains(&"-m".to_string()));
    assert!(a.contains(&"hello".to_string()));
}

#[test]
fn status_porcelain_v2() {
    let mut c = StatusCommand::new();
    c.format(StatusFormat::PorcelainV2).branch();
    assert_eq!(args_of(&c), vec!["status", "--porcelain=v2", "--branch"]);
}

#[test]
fn status_porcelain_v1_branch_null_terminated() {
    let mut c = StatusCommand::new();
    c.format(StatusFormat::PorcelainV1)
        .branch()
        .null_terminate();
    assert_eq!(
        args_of(&c),
        vec!["status", "--porcelain=v1", "--branch", "-z"]
    );
}

#[test]
fn log_with_limits_and_paths() {
    let mut c = LogCommand::new();
    c.max_count(5).oneline().revision("HEAD").path("src/lib.rs");
    assert_eq!(
        args_of(&c),
        vec!["log", "-n5", "--oneline", "HEAD", "--", "src/lib.rs"]
    );
}

#[test]
fn diff_cached_numstat() {
    let mut c = DiffCommand::new();
    c.cached().numstat();
    assert_eq!(args_of(&c), vec!["diff", "--cached", "--numstat"]);
}

#[test]
fn diff_numstat_null_terminated() {
    let mut c = DiffCommand::new();
    c.numstat().null_terminate();
    assert_eq!(args_of(&c), vec!["diff", "--numstat", "-z"]);
}

#[test]
fn diff_stat() {
    let mut c = DiffCommand::new();
    c.stat();
    assert_eq!(args_of(&c), vec!["diff", "--stat"]);
}

#[test]
fn show_object_with_format() {
    let mut c = ShowCommand::new();
    c.object("HEAD").format("%H %s").no_patch();
    assert_eq!(
        args_of(&c),
        vec!["show", "--format=%H %s", "--no-patch", "HEAD"]
    );
}

#[test]
fn branch_delete() {
    let mut c = BranchCommand::new();
    c.delete("old");
    assert_eq!(args_of(&c), vec!["branch", "-d", "old"]);
}

#[test]
fn branch_force_delete() {
    // force_delete upgrades the delete flag to -D; it must not emit both.
    let mut c = BranchCommand::new();
    c.delete("old").force_delete();
    let a = args_of(&c);
    assert_eq!(a, vec!["branch", "-D", "old"]);
    assert!(!a.contains(&"-d".to_string()));
}

#[test]
fn branch_rename() {
    let mut c = BranchCommand::new();
    c.rename("a", "b");
    assert_eq!(args_of(&c), vec!["branch", "-m", "a", "b"]);
}

#[test]
fn checkout_create_branch() {
    let mut c = CheckoutCommand::new();
    c.create("feature/x");
    assert_eq!(args_of(&c), vec!["checkout", "-b", "feature/x"]);
}

#[test]
fn switch_create() {
    let mut c = SwitchCommand::new();
    c.create("dev");
    assert_eq!(args_of(&c), vec!["switch", "-c", "dev"]);
}

#[test]
fn merge_abort_short_circuits() {
    let mut c = MergeCommand::new();
    c.no_ff().commit_ref("other").abort();
    assert_eq!(args_of(&c), vec!["merge", "--abort"]);
}

#[test]
fn rebase_onto() {
    let mut c = RebaseCommand::new();
    c.onto("main").upstream("develop");
    assert_eq!(args_of(&c), vec!["rebase", "--onto", "main", "develop"]);
}

#[test]
fn pull_rebase_mode() {
    let mut c = PullCommand::new();
    c.rebase_mode("merges").remote("origin");
    assert_eq!(args_of(&c), vec!["pull", "--rebase=merges", "origin"]);
}

#[test]
fn push_set_upstream() {
    let mut c = PushCommand::new();
    c.set_upstream()
        .remote("origin")
        .refspec("HEAD:refs/heads/feat");
    assert_eq!(
        args_of(&c),
        vec!["push", "--set-upstream", "origin", "HEAD:refs/heads/feat",]
    );
}

#[test]
fn fetch_prune_depth() {
    let mut c = FetchCommand::new();
    c.prune().depth(10).remote("origin");
    assert_eq!(
        args_of(&c),
        vec!["fetch", "--prune", "--depth=10", "origin"]
    );
}

#[test]
fn remote_add() {
    let c = RemoteCommand::add("upstream", "https://example.com/up.git");
    assert_eq!(
        args_of(&c),
        vec!["remote", "add", "upstream", "https://example.com/up.git"]
    );
}

#[test]
fn remote_list_verbose() {
    let c = RemoteCommand::list_verbose();
    assert_eq!(args_of(&c), vec!["remote", "-v"]);
}

#[test]
fn tag_annotated() {
    let mut c = TagCommand::new();
    c.message("v1").name("v1.0.0");
    let a = args_of(&c);
    assert!(a.contains(&"-a".to_string()));
    assert!(a.contains(&"-m".to_string()));
    assert!(a.contains(&"v1".to_string()));
    assert!(a.contains(&"v1.0.0".to_string()));
}

#[test]
fn stash_push_with_message() {
    let mut c = StashCommand::push();
    c.message("wip").include_untracked().keep_index();
    assert_eq!(
        args_of(&c),
        vec![
            "stash",
            "push",
            "--include-untracked",
            "--keep-index",
            "-m",
            "wip",
        ]
    );
}

#[test]
fn stash_list_action() {
    let c = StashCommand {
        executor: Default::default(),
        action: StashAction::List,
    };
    assert_eq!(args_of(&c), vec!["stash", "list"]);
}

#[test]
fn reset_hard_to_commit() {
    let mut c = ResetCommand::new();
    c.mode(ResetMode::Hard).commit("HEAD~1");
    assert_eq!(args_of(&c), vec!["reset", "--hard", "HEAD~1"]);
}

#[test]
fn restore_staged_path() {
    let mut c = RestoreCommand::new();
    c.staged().path("Cargo.toml");
    assert_eq!(args_of(&c), vec!["restore", "--staged", "--", "Cargo.toml"]);
}

#[test]
fn rm_cached_recursive() {
    let mut c = RmCommand::new();
    c.cached().recursive().path("target");
    assert_eq!(args_of(&c), vec!["rm", "--cached", "-r", "--", "target"]);
}

#[test]
fn mv_source_dest() {
    let c = MvCommand::new("a.txt", "b.txt");
    assert_eq!(args_of(&c), vec!["mv", "a.txt", "b.txt"]);
}

#[test]
fn rev_parse_head_short() {
    let mut c = RevParseCommand::new();
    c.short_len(7).arg_str("HEAD");
    assert_eq!(args_of(&c), vec!["rev-parse", "--short=7", "HEAD"]);
}

#[test]
fn ls_files_cached_with_path() {
    let mut c = LsFilesCommand::new();
    c.cached().path("src");
    assert_eq!(args_of(&c), vec!["ls-files", "--cached", "--", "src"]);
}

#[test]
fn ls_tree_recurse() {
    let mut c = LsTreeCommand::new("HEAD");
    c.recurse().name_only();
    assert_eq!(args_of(&c), vec!["ls-tree", "-r", "--name-only", "HEAD"]);
}

#[test]
fn ls_tree_path_uses_separator() {
    let mut c = LsTreeCommand::new("HEAD");
    c.path("--suspicious-file");
    assert_eq!(
        args_of(&c),
        vec!["ls-tree", "HEAD", "--", "--suspicious-file"]
    );
}

#[test]
fn cat_file_pretty_print() {
    let c = CatFileCommand::pretty_print("HEAD");
    assert_eq!(args_of(&c), vec!["cat-file", "-p", "HEAD"]);
}

#[test]
fn hash_object_write() {
    let mut c = HashObjectCommand::new();
    c.write().path("/tmp/blob");
    assert_eq!(args_of(&c), vec!["hash-object", "-w", "--", "/tmp/blob"]);
}

#[test]
fn hash_object_path_uses_separator() {
    let mut c = HashObjectCommand::new();
    c.path("--suspicious-file");
    assert_eq!(args_of(&c), vec!["hash-object", "--", "--suspicious-file"]);
}

#[test]
fn update_ref_set() {
    let mut c = UpdateRefCommand::new();
    c.ref_name("refs/heads/main").new_value("abc123");
    assert_eq!(args_of(&c), vec!["update-ref", "refs/heads/main", "abc123"]);
}

#[test]
fn update_ref_delete() {
    let mut c = UpdateRefCommand::new();
    c.ref_name("refs/heads/gone").delete();
    assert_eq!(args_of(&c), vec!["update-ref", "-d", "refs/heads/gone"]);
}

#[test]
fn for_each_ref_pattern() {
    let mut c = ForEachRefCommand::new();
    c.pattern("refs/heads/*").format("%(refname:short)");
    assert_eq!(
        args_of(&c),
        vec!["for-each-ref", "--format=%(refname:short)", "refs/heads/*",]
    );
}

#[test]
fn describe_with_tags_and_dirty() {
    let mut c = DescribeCommand::new();
    c.tags().long().dirty_mark("-wip").commit("HEAD");
    assert_eq!(
        args_of(&c),
        vec!["describe", "--tags", "--long", "--dirty=-wip", "HEAD"]
    );
}

#[test]
fn show_ref_heads_pattern() {
    let mut c = ShowRefCommand::new();
    c.heads().pattern("main");
    assert_eq!(args_of(&c), vec!["show-ref", "--heads", "main"]);
}

#[test]
fn symbolic_ref_read_head() {
    let mut c = SymbolicRefCommand::read("HEAD");
    c.short();
    assert_eq!(args_of(&c), vec!["symbolic-ref", "--short", "HEAD"]);
}

#[test]
fn symbolic_ref_set_with_reason() {
    let mut c = SymbolicRefCommand::set("HEAD", "refs/heads/main");
    c.reason("switching branches");
    assert_eq!(
        args_of(&c),
        vec![
            "symbolic-ref",
            "-m",
            "switching branches",
            "HEAD",
            "refs/heads/main",
        ]
    );
}

#[test]
fn symbolic_ref_delete() {
    let mut c = SymbolicRefCommand::delete("FETCH_HEAD");
    c.quiet();
    assert_eq!(
        args_of(&c),
        vec!["symbolic-ref", "--delete", "-q", "FETCH_HEAD"]
    );
}

#[test]
fn escape_hatch_arg_appends_after_typed_args() {
    // `arg()` goes into the executor's raw_args, which the executor appends
    // after the typed args when spawning. Build only yields the typed args;
    // the integration test in porcelain.rs verifies combined execution.
    let mut c = StatusCommand::new();
    c.arg("--porcelain=v2");
    assert_eq!(args_of(&c), vec!["status"]);
    assert_eq!(c.executor.raw_args, vec!["--porcelain=v2"]);
}

#[test]
fn format_patch_range() {
    let mut c = FormatPatchCommand::new();
    c.rev_spec("HEAD~3..HEAD");
    assert_eq!(args_of(&c), vec!["format-patch", "HEAD~3..HEAD"]);
}

#[test]
fn format_patch_output_dir_numbered_signoff() {
    let mut c = FormatPatchCommand::new();
    c.rev_spec("HEAD~1..HEAD")
        .output_dir("/tmp/p")
        .numbered()
        .signoff();
    assert_eq!(
        args_of(&c),
        vec![
            "format-patch",
            "-n",
            "--signoff",
            "-o",
            "/tmp/p",
            "HEAD~1..HEAD"
        ]
    );
}

#[test]
fn apply_single_patch() {
    let mut c = ApplyCommand::new();
    c.patch("/tmp/p/0001-fix.patch");
    assert_eq!(args_of(&c), vec!["apply", "/tmp/p/0001-fix.patch"]);
}

#[test]
fn apply_check_reverse_three_way_index_cached_strip() {
    let mut c = ApplyCommand::new();
    c.patch("a.patch")
        .patch("b.patch")
        .check()
        .reverse()
        .three_way()
        .index()
        .cached()
        .strip(2);
    assert_eq!(
        args_of(&c),
        vec![
            "apply",
            "--check",
            "--reverse",
            "--3way",
            "--index",
            "--cached",
            "-p2",
            "a.patch",
            "b.patch"
        ]
    );
}

#[test]
fn am_single_mailbox() {
    let mut c = AmCommand::new();
    c.mailbox("/tmp/p/0001-fix.patch");
    assert_eq!(args_of(&c), vec!["am", "/tmp/p/0001-fix.patch"]);
}

#[test]
fn am_signoff_three_way_keep_cr_strip() {
    let mut c = AmCommand::new();
    c.mailbox("a.patch")
        .mailbox("b.patch")
        .signoff()
        .three_way()
        .keep_cr()
        .strip(1);
    assert_eq!(
        args_of(&c),
        vec![
            "am",
            "--signoff",
            "--3way",
            "--keep-cr",
            "-p1",
            "a.patch",
            "b.patch"
        ]
    );
}

#[test]
fn am_session_controls_replace_everything_else() {
    let mut abort = AmCommand::new();
    abort.mailbox("a.patch").signoff().abort();
    assert_eq!(args_of(&abort), vec!["am", "--abort"]);

    let mut cont = AmCommand::new();
    cont.mailbox("a.patch").cont();
    assert_eq!(args_of(&cont), vec!["am", "--continue"]);

    let mut skip = AmCommand::new();
    skip.mailbox("a.patch").skip();
    assert_eq!(args_of(&skip), vec!["am", "--skip"]);
}

#[test]
fn verify_commit_single() {
    let mut c = VerifyCommitCommand::new();
    c.commit("HEAD");
    assert_eq!(args_of(&c), vec!["verify-commit", "HEAD"]);
}

#[test]
fn verify_commit_raw_verbose_multiple() {
    let mut c = VerifyCommitCommand::new();
    c.commit("HEAD").commit("HEAD~1").raw().verbose();
    assert_eq!(
        args_of(&c),
        vec!["verify-commit", "--raw", "-v", "HEAD", "HEAD~1"]
    );
}

#[test]
fn verify_tag_single() {
    let mut c = VerifyTagCommand::new();
    c.tag("v1.0.0");
    assert_eq!(args_of(&c), vec!["verify-tag", "v1.0.0"]);
}

#[test]
fn verify_tag_raw_verbose_multiple() {
    let mut c = VerifyTagCommand::new();
    c.tag("v1.0.0").tag("v1.1.0").raw().verbose();
    assert_eq!(
        args_of(&c),
        vec!["verify-tag", "--raw", "-v", "v1.0.0", "v1.1.0"]
    );
}

#[test]
fn cherry_upstream_only() {
    let mut c = CherryCommand::new();
    c.upstream("origin/main");
    assert_eq!(args_of(&c), vec!["cherry", "origin/main"]);
}

#[test]
fn cherry_verbose_with_head_and_limit() {
    let mut c = CherryCommand::new();
    c.upstream("origin/main")
        .head("feature")
        .limit("v1.0")
        .verbose();
    assert_eq!(
        args_of(&c),
        vec!["cherry", "-v", "origin/main", "feature", "v1.0"]
    );
}

#[test]
fn cherry_defaults_to_the_configured_upstream() {
    let c = CherryCommand::new();
    assert_eq!(args_of(&c), vec!["cherry"]);
}

#[test]
fn blame_plain_file() {
    let mut c = BlameCommand::new();
    c.file("src/lib.rs");
    assert_eq!(args_of(&c), vec!["blame", "--", "src/lib.rs"]);
}

#[test]
fn blame_porcelain_with_line_range_and_rev() {
    let mut c = BlameCommand::new();
    c.file("src/lib.rs").rev("HEAD~3").lines(10, 20).porcelain();
    assert_eq!(
        args_of(&c),
        vec![
            "blame",
            "--porcelain",
            "-L",
            "10,20",
            "HEAD~3",
            "--",
            "src/lib.rs"
        ]
    );
}

#[test]
fn blame_line_porcelain_with_detection_options() {
    let mut c = BlameCommand::new();
    c.file("src/lib.rs")
        .line_porcelain()
        .ignore_whitespace()
        .detect_moved()
        .detect_copied();
    assert_eq!(
        args_of(&c),
        vec![
            "blame",
            "--line-porcelain",
            "-w",
            "-M",
            "-C",
            "--",
            "src/lib.rs"
        ]
    );
}

#[test]
fn blame_show_email_keeps_the_human_format() {
    let mut c = BlameCommand::new();
    c.file("src/lib.rs").show_email();
    assert_eq!(args_of(&c), vec!["blame", "-e", "--", "src/lib.rs"]);
}

#[test]
fn gc_bare_is_just_gc() {
    let c = GcCommand::new();
    assert_eq!(args_of(&c), vec!["gc"]);
}

#[test]
fn gc_aggressive_auto() {
    let mut c = GcCommand::new();
    c.aggressive().auto();
    assert_eq!(args_of(&c), vec!["gc", "--aggressive", "--auto"]);
}

#[test]
fn gc_prune_date() {
    let mut c = GcCommand::new();
    c.prune("now");
    assert_eq!(args_of(&c), vec!["gc", "--prune=now"]);
}

#[test]
fn gc_no_prune_after_prune_wins() {
    let mut c = GcCommand::new();
    c.prune("2.weeks.ago").no_prune();
    assert_eq!(args_of(&c), vec!["gc", "--no-prune"]);
}
