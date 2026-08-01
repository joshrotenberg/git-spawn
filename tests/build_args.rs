//! Pure argv assertions: verifies each command builds the right argument vector
//! without spawning `git`.

use git_spawn::command::{
    archive::ArchiveFormat,
    interpret_trailers::{TrailerIfExists, TrailerIfMissing, TrailerWhere},
    maintenance::{MaintenanceSchedule, MaintenanceTask},
    reset::ResetMode,
    stash::StashCommand,
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
    let c = StashCommand::list();
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
fn range_diff_two_ranges() {
    let mut c = RangeDiffCommand::new();
    c.rev("main..v1").rev("main..v2");
    assert_eq!(args_of(&c), vec!["range-diff", "main..v1", "main..v2"]);
}

#[test]
fn range_diff_base_form_with_options() {
    let mut c = RangeDiffCommand::new();
    c.rev("main")
        .rev("v1")
        .rev("v2")
        .no_dual_color()
        .creation_factor(90);
    assert_eq!(
        args_of(&c),
        vec![
            "range-diff",
            "--no-dual-color",
            "--creation-factor=90",
            "main",
            "v1",
            "v2"
        ]
    );
}

#[test]
fn range_diff_symmetric_form_left_and_right_only() {
    let mut left = RangeDiffCommand::new();
    left.rev("v1...v2").left_only();
    assert_eq!(args_of(&left), vec!["range-diff", "--left-only", "v1...v2"]);

    let mut right = RangeDiffCommand::new();
    right.rev("v1...v2").right_only();
    assert_eq!(
        args_of(&right),
        vec!["range-diff", "--right-only", "v1...v2"]
    );
}

#[test]
fn merge_base_two_commits() {
    let mut c = MergeBaseCommand::new();
    c.commit("main").commit("feature");
    assert_eq!(args_of(&c), vec!["merge-base", "main", "feature"]);
}

#[test]
fn merge_base_all_with_several_commits() {
    let mut c = MergeBaseCommand::new();
    c.commits(["main", "feature", "topic"]).all();
    assert_eq!(
        args_of(&c),
        vec!["merge-base", "--all", "main", "feature", "topic"]
    );
}

#[test]
fn merge_base_is_ancestor_puts_the_flag_first() {
    let mut c = MergeBaseCommand::new();
    c.commit("v1.0").is_ancestor().commit("main");
    assert_eq!(
        args_of(&c),
        vec!["merge-base", "--is-ancestor", "v1.0", "main"]
    );
}

#[test]
fn merge_base_fork_point_with_only_a_ref() {
    let mut c = MergeBaseCommand::new();
    c.fork_point().commit("main");
    assert_eq!(args_of(&c), vec!["merge-base", "--fork-point", "main"]);
}

#[test]
fn merge_base_fork_point_with_a_ref_and_a_commit() {
    let mut c = MergeBaseCommand::new();
    c.fork_point().commit("main").commit("feature");
    assert_eq!(
        args_of(&c),
        vec!["merge-base", "--fork-point", "main", "feature"]
    );
}

#[test]
fn interpret_trailers_single_trailer() {
    let mut c = InterpretTrailersCommand::new();
    c.trailer("Signed-off-by", "A U Thor <author@example.com>")
        .file("MSG");
    assert_eq!(
        args_of(&c),
        vec![
            "interpret-trailers",
            "--trailer",
            "Signed-off-by: A U Thor <author@example.com>",
            "MSG"
        ]
    );
}

#[test]
fn maintenance_run_bare() {
    let c = MaintenanceCommand::run();
    assert_eq!(args_of(&c), vec!["maintenance", "run"]);
}

#[test]
fn maintenance_run_quiet_with_repeated_tasks() {
    let mut c = MaintenanceCommand::run();
    c.quiet()
        .task(MaintenanceTask::CommitGraph)
        .task(MaintenanceTask::IncrementalRepack)
        .task_raw("bespoke-task");
    assert_eq!(
        args_of(&c),
        vec![
            "maintenance",
            "run",
            "--quiet",
            "--task=commit-graph",
            "--task=incremental-repack",
            "--task=bespoke-task",
        ]
    );
}

#[test]
fn bundle_create_all_quiet_and_versioned() {
    let mut c = BundleCommand::create("/tmp/r.bundle");
    c.quiet().version(2).all();
    assert_eq!(
        args_of(&c),
        vec![
            "bundle",
            "create",
            "--quiet",
            "--version=2",
            "/tmp/r.bundle",
            "--all"
        ]
    );
}

#[test]
fn interpret_trailers_placement_and_conflict_actions() {
    let mut c = InterpretTrailersCommand::new();
    c.in_place()
        .trim_empty()
        .placement(TrailerWhere::Start)
        .if_exists(TrailerIfExists::Replace)
        .if_missing(TrailerIfMissing::DoNothing)
        .trailer_raw("Reviewed-by: R Viewer <r@example.com>")
        .file("a.txt")
        .file("b.txt");
    assert_eq!(
        args_of(&c),
        vec![
            "interpret-trailers",
            "--in-place",
            "--trim-empty",
            "--where=start",
            "--if-exists=replace",
            "--if-missing=doNothing",
            "--trailer",
            "Reviewed-by: R Viewer <r@example.com>",
            "a.txt",
            "b.txt"
        ]
    );
}

#[test]
fn interpret_trailers_parse_shorthand_and_reading_options() {
    let mut c = InterpretTrailersCommand::new();
    c.parse().no_divider().file("MSG");
    assert_eq!(
        args_of(&c),
        vec!["interpret-trailers", "--parse", "--no-divider", "MSG"]
    );

    let mut spelled_out = InterpretTrailersCommand::new();
    spelled_out
        .only_trailers()
        .only_input()
        .unfold()
        .file("MSG");
    assert_eq!(
        args_of(&spelled_out),
        vec![
            "interpret-trailers",
            "--only-trailers",
            "--only-input",
            "--unfold",
            "MSG"
        ]
    );
}

#[test]
fn clean_dry_run_with_directories() {
    let mut c = CleanCommand::new();
    c.dry_run().directories();
    assert_eq!(args_of(&c), vec!["clean", "--dry-run", "-d"]);
}

#[test]
fn clean_force_including_ignored_files() {
    let mut c = CleanCommand::new();
    c.force().directories().ignored();
    assert_eq!(args_of(&c), vec!["clean", "--force", "-d", "-x"]);
}

#[test]
fn clean_pathspecs_follow_a_separator() {
    let mut c = CleanCommand::new();
    c.force().paths(["build", "target"]).path("dist");
    assert_eq!(
        args_of(&c),
        vec!["clean", "--force", "--", "build", "target", "dist"]
    );
}

#[test]
fn revert_single_commit_no_edit() {
    let mut c = RevertCommand::new();
    c.commit("HEAD").no_edit();
    assert_eq!(args_of(&c), vec!["revert", "--no-edit", "HEAD"]);
}

#[test]
fn revert_merge_without_committing() {
    let mut c = RevertCommand::new();
    c.commit("abc1234").no_commit().mainline(1);
    assert_eq!(
        args_of(&c),
        vec!["revert", "--no-commit", "--mainline", "1", "abc1234"]
    );
}

#[test]
fn revert_multiple_commits_keep_their_order() {
    let mut c = RevertCommand::new();
    c.commit("aaa1111").commit("bbb2222").no_edit();
    assert_eq!(
        args_of(&c),
        vec!["revert", "--no-edit", "aaa1111", "bbb2222"]
    );
}

#[test]
fn revert_session_action_drops_the_other_arguments() {
    let mut c = RevertCommand::new();
    c.commit("HEAD").no_commit().abort();
    assert_eq!(args_of(&c), vec!["revert", "--abort"]);
}

#[test]
fn shortlog_plain_range() {
    let mut c = ShortlogCommand::new();
    c.rev("v1.0..HEAD");
    assert_eq!(args_of(&c), vec!["shortlog", "v1.0..HEAD"]);
}

#[test]
fn shortlog_summary_numbered_with_emails() {
    let mut c = ShortlogCommand::new();
    c.rev("HEAD").summary().numbered().email();
    assert_eq!(args_of(&c), vec!["shortlog", "-s", "-n", "-e", "HEAD"]);
}

#[test]
fn shortlog_committer_grouping_with_a_wrap_width() {
    let mut c = ShortlogCommand::new();
    c.rev("HEAD").committer().wrap(0);
    assert_eq!(args_of(&c), vec!["shortlog", "-c", "-w0", "HEAD"]);
}

#[test]
fn shortlog_paths_follow_the_separator() {
    let mut c = ShortlogCommand::new();
    c.rev("main").rev("topic").paths(["src", "tests"]);
    assert_eq!(
        args_of(&c),
        vec!["shortlog", "main", "topic", "--", "src", "tests"]
    );
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

#[test]
fn fsck_bare() {
    let c = FsckCommand::new();
    assert_eq!(args_of(&c), vec!["fsck"]);
}

#[test]
fn fsck_full_with_unreachable() {
    let mut c = FsckCommand::new();
    c.full().unreachable();
    assert_eq!(args_of(&c), vec!["fsck", "--full", "--unreachable"]);
}

#[test]
fn fsck_no_dangling() {
    let mut c = FsckCommand::new();
    c.no_dangling();
    assert_eq!(args_of(&c), vec!["fsck", "--no-dangling"]);
}

#[test]
fn fsck_dangling_after_no_dangling_wins() {
    let mut c = FsckCommand::new();
    c.no_dangling().dangling();
    assert_eq!(args_of(&c), vec!["fsck", "--dangling"]);
}

#[test]
fn maintenance_run_schedule_replaces_auto() {
    // git rejects --auto alongside --schedule, so the two share one field and
    // the last call wins rather than emitting a pair git will refuse.
    let mut c = MaintenanceCommand::run();
    c.auto().schedule(MaintenanceSchedule::Weekly);
    assert_eq!(args_of(&c), vec!["maintenance", "run", "--schedule=weekly"]);

    let mut c = MaintenanceCommand::run();
    c.schedule(MaintenanceSchedule::Hourly).auto();
    assert_eq!(args_of(&c), vec!["maintenance", "run", "--auto"]);
}

#[test]
fn maintenance_register_with_config_file() {
    let mut c = MaintenanceCommand::register();
    c.config_file("/tmp/gitconfig");
    assert_eq!(
        args_of(&c),
        vec!["maintenance", "register", "--config-file", "/tmp/gitconfig"]
    );
}

#[test]
fn maintenance_unregister_forced() {
    let mut c = MaintenanceCommand::unregister();
    c.force().config_file("/tmp/gitconfig");
    assert_eq!(
        args_of(&c),
        vec![
            "maintenance",
            "unregister",
            "--force",
            "--config-file",
            "/tmp/gitconfig",
        ]
    );
}

#[test]
fn archive_bare_is_the_tree_ish_alone() {
    let c = ArchiveCommand::new("HEAD");
    assert_eq!(args_of(&c), vec!["archive", "HEAD"]);
}

#[test]
fn archive_format_and_prefix_precede_the_tree_ish() {
    let mut c = ArchiveCommand::new("v1.0");
    c.format(ArchiveFormat::Zip).prefix("proj/");
    assert_eq!(
        args_of(&c),
        vec!["archive", "--format=zip", "--prefix=proj/", "v1.0"]
    );
}

#[test]
fn archive_output_then_tree_ish_then_paths() {
    let mut c = ArchiveCommand::new("HEAD");
    c.output("/tmp/out.tar").path("src").path("README.md");
    assert_eq!(
        args_of(&c),
        vec!["archive", "-o", "/tmp/out.tar", "HEAD", "src", "README.md"]
    );
}

#[test]
fn archive_format_raw_passes_the_name_through() {
    let mut c = ArchiveCommand::new("HEAD");
    c.format_raw("tar.zst");
    assert_eq!(args_of(&c), vec!["archive", "--format=tar.zst", "HEAD"]);
}

#[test]
fn archive_format_raw_after_format_wins() {
    let mut c = ArchiveCommand::new("HEAD");
    c.format(ArchiveFormat::TarGz).format_raw("zip");
    assert_eq!(args_of(&c), vec!["archive", "--format=zip", "HEAD"]);
}

#[test]
fn bundle_create_keeps_rev_list_args_after_the_file() {
    let mut c = BundleCommand::create("/tmp/r.bundle");
    c.rev("main").rev("v1.0..topic");
    assert_eq!(
        args_of(&c),
        vec!["bundle", "create", "/tmp/r.bundle", "main", "v1.0..topic"]
    );
}

#[test]
fn bundle_create_progress_after_quiet_wins() {
    let mut c = BundleCommand::create("/tmp/r.bundle");
    c.quiet().progress().all();
    assert_eq!(
        args_of(&c),
        vec!["bundle", "create", "--progress", "/tmp/r.bundle", "--all"]
    );
}

#[test]
fn bundle_verify_quiet() {
    let mut c = BundleCommand::verify("/tmp/r.bundle");
    c.quiet();
    assert_eq!(
        args_of(&c),
        vec!["bundle", "verify", "--quiet", "/tmp/r.bundle"]
    );
}

#[test]
fn bundle_list_heads_filtered_by_ref() {
    let mut c = BundleCommand::list_heads("/tmp/r.bundle");
    c.ref_name("refs/heads/main");
    assert_eq!(
        args_of(&c),
        vec!["bundle", "list-heads", "/tmp/r.bundle", "refs/heads/main"]
    );
}

#[test]
fn bundle_unbundle_with_progress_and_refs() {
    let mut c = BundleCommand::unbundle("/tmp/r.bundle");
    c.progress().ref_name("refs/heads/main");
    assert_eq!(
        args_of(&c),
        vec![
            "bundle",
            "unbundle",
            "--progress",
            "/tmp/r.bundle",
            "refs/heads/main"
        ]
    );
}

#[test]
fn rerere_status_and_diff() {
    assert_eq!(args_of(&RerereCommand::status()), vec!["rerere", "status"]);
    assert_eq!(args_of(&RerereCommand::diff()), vec!["rerere", "diff"]);
}

#[test]
fn rerere_maintenance_actions() {
    assert_eq!(args_of(&RerereCommand::gc()), vec!["rerere", "gc"]);
    assert_eq!(args_of(&RerereCommand::clear()), vec!["rerere", "clear"]);
}

#[test]
fn rerere_forget_takes_its_pathspec_from_the_constructor() {
    let c = RerereCommand::forget("README");
    assert_eq!(args_of(&c), vec!["rerere", "forget", "README"]);
}

#[test]
fn rerere_forget_appends_further_pathspecs() {
    let mut c = RerereCommand::forget("README");
    c.pathspec("src/lib.rs").pathspec("docs/");
    assert_eq!(
        args_of(&c),
        vec!["rerere", "forget", "README", "src/lib.rs", "docs/"]
    );
}

#[test]
fn rerere_pathspec_is_ignored_by_the_other_actions() {
    let mut c = RerereCommand::status();
    c.pathspec("README");
    assert_eq!(args_of(&c), vec!["rerere", "status"]);
}

#[test]
fn sparse_checkout_list_and_disable() {
    assert_eq!(
        args_of(&SparseCheckoutCommand::list()),
        vec!["sparse-checkout", "list"]
    );
    assert_eq!(
        args_of(&SparseCheckoutCommand::disable()),
        vec!["sparse-checkout", "disable"]
    );
}

#[test]
fn sparse_checkout_init_toggles() {
    assert_eq!(
        args_of(&SparseCheckoutCommand::init()),
        vec!["sparse-checkout", "init"]
    );

    let mut cone = SparseCheckoutCommand::init();
    cone.cone().sparse_index();
    assert_eq!(
        args_of(&cone),
        vec!["sparse-checkout", "init", "--cone", "--sparse-index"]
    );

    let mut plain = SparseCheckoutCommand::init();
    plain.no_cone().no_sparse_index();
    assert_eq!(
        args_of(&plain),
        vec!["sparse-checkout", "init", "--no-cone", "--no-sparse-index"]
    );
}

#[test]
fn sparse_checkout_set_takes_its_pattern_from_the_constructor() {
    let c = SparseCheckoutCommand::set("src");
    assert_eq!(args_of(&c), vec!["sparse-checkout", "set", "src"]);
}

#[test]
fn sparse_checkout_set_appends_further_patterns_after_its_options() {
    let mut c = SparseCheckoutCommand::set("src");
    c.pattern("docs").no_cone().skip_checks();
    assert_eq!(
        args_of(&c),
        vec![
            "sparse-checkout",
            "set",
            "--no-cone",
            "--skip-checks",
            "src",
            "docs"
        ]
    );
}

#[test]
fn sparse_checkout_add_appends_further_patterns() {
    let mut c = SparseCheckoutCommand::add("src");
    c.pattern("docs").skip_checks();
    assert_eq!(
        args_of(&c),
        vec!["sparse-checkout", "add", "--skip-checks", "src", "docs"]
    );
}

#[test]
fn sparse_checkout_options_are_ignored_by_the_actions_they_do_not_apply_to() {
    // `list` takes none of them, and `add` takes neither cone nor sparse-index.
    let mut list = SparseCheckoutCommand::list();
    list.pattern("src").cone().sparse_index().skip_checks();
    assert_eq!(args_of(&list), vec!["sparse-checkout", "list"]);

    let mut add = SparseCheckoutCommand::add("src");
    add.cone().sparse_index();
    assert_eq!(args_of(&add), vec!["sparse-checkout", "add", "src"]);
}

#[test]
fn ls_remote_defaults_to_the_configured_remote() {
    let c = LsRemoteCommand::new();
    assert_eq!(args_of(&c), vec!["ls-remote"]);
}

#[test]
fn ls_remote_named_remote() {
    let c = LsRemoteCommand::remote("origin");
    assert_eq!(args_of(&c), vec!["ls-remote", "origin"]);
}

#[test]
fn ls_remote_heads_and_tags_with_patterns() {
    let mut c = LsRemoteCommand::remote("origin");
    c.heads().tags().refs().pattern("v1.*").pattern("main");
    assert_eq!(
        args_of(&c),
        vec![
            "ls-remote",
            "--heads",
            "--tags",
            "--refs",
            "origin",
            "v1.*",
            "main"
        ]
    );
}

#[test]
fn ls_remote_symref_exit_code_and_quiet() {
    let mut c = LsRemoteCommand::new();
    c.repository("https://example.com/foo.git")
        .symref()
        .exit_code()
        .quiet();
    assert_eq!(
        args_of(&c),
        vec![
            "ls-remote",
            "--symref",
            "--exit-code",
            "-q",
            "https://example.com/foo.git"
        ]
    );
}

#[test]
fn ls_remote_flags_precede_the_positional_repository() {
    let mut c = LsRemoteCommand::new();
    c.pattern("HEAD").repository("origin").heads();
    assert_eq!(args_of(&c), vec!["ls-remote", "--heads", "origin", "HEAD"]);
}

#[test]
fn name_rev_single_rev() {
    let mut c = NameRevCommand::new();
    c.rev("HEAD");
    assert_eq!(args_of(&c), vec!["name-rev", "HEAD"]);
}

#[test]
fn name_rev_name_only_with_several_revs() {
    let mut c = NameRevCommand::new();
    c.revs(["HEAD", "HEAD~1"]).name_only();
    assert_eq!(
        args_of(&c),
        vec!["name-rev", "--name-only", "HEAD", "HEAD~1"]
    );
}

#[test]
fn name_rev_tags_puts_the_flag_before_the_rev() {
    let mut c = NameRevCommand::new();
    c.rev("HEAD").tags();
    assert_eq!(args_of(&c), vec!["name-rev", "--tags", "HEAD"]);
}

#[test]
fn name_rev_refs_repeat_into_separate_flags() {
    let mut c = NameRevCommand::new();
    c.refs("refs/heads/*").refs("refs/tags/v*").rev("HEAD");
    assert_eq!(
        args_of(&c),
        vec![
            "name-rev",
            "--refs=refs/heads/*",
            "--refs=refs/tags/v*",
            "HEAD"
        ]
    );
}

#[test]
fn name_rev_no_revs_builds_the_bare_subcommand() {
    let c = NameRevCommand::new();
    assert_eq!(args_of(&c), vec!["name-rev"]);
}

#[test]
fn var_named_variable() {
    let c = VarCommand::get("GIT_AUTHOR_IDENT");
    assert_eq!(args_of(&c), vec!["var", "GIT_AUTHOR_IDENT"]);
}

#[test]
fn var_list() {
    let c = VarCommand::list();
    assert_eq!(args_of(&c), vec!["var", "-l"]);
}

#[test]
fn var_name_and_list_together_build_both_and_are_rejected_at_execute() {
    let mut c = VarCommand::list();
    c.name = Some("GIT_EDITOR".to_string());
    assert_eq!(args_of(&c), vec!["var", "-l", "GIT_EDITOR"]);
}

#[test]
fn version_is_a_top_level_flag_not_a_subcommand() {
    let c = VersionCommand::new();
    assert_eq!(args_of(&c), vec!["--version"]);
}

#[test]
fn version_build_options_follows_the_version_flag() {
    let mut c = VersionCommand::new();
    c.build_options();
    assert_eq!(args_of(&c), vec!["--version", "--build-options"]);
}

#[test]
fn count_objects_plain() {
    let c = CountObjectsCommand::new();
    assert_eq!(args_of(&c), vec!["count-objects"]);
}

#[test]
fn count_objects_verbose() {
    let mut c = CountObjectsCommand::new();
    c.verbose();
    assert_eq!(args_of(&c), vec!["count-objects", "-v"]);
}

#[test]
fn count_objects_human_readable() {
    let mut c = CountObjectsCommand::new();
    c.human_readable();
    assert_eq!(args_of(&c), vec!["count-objects", "-H"]);
}

#[test]
fn count_objects_verbose_precedes_human_readable_whatever_the_call_order() {
    let mut c = CountObjectsCommand::new();
    c.human_readable().verbose();
    assert_eq!(args_of(&c), vec!["count-objects", "-v", "-H"]);
}

#[test]
fn check_ignore_paths_follow_a_separator() {
    let mut c = CheckIgnoreCommand::new();
    c.paths(["build/", "-generated.log"]);
    assert_eq!(
        args_of(&c),
        vec!["check-ignore", "--", "build/", "-generated.log"]
    );
}

#[test]
fn check_ignore_verbose_non_matching_and_no_index() {
    let mut c = CheckIgnoreCommand::new();
    c.path("app.log").verbose().non_matching().no_index();
    assert_eq!(
        args_of(&c),
        vec!["check-ignore", "-v", "-n", "--no-index", "--", "app.log"]
    );
}

#[test]
fn check_ignore_quiet() {
    let mut c = CheckIgnoreCommand::new();
    c.path("app.log").quiet();
    assert_eq!(args_of(&c), vec!["check-ignore", "-q", "--", "app.log"]);
}
