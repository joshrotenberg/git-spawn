//! Pure argv assertions for advanced commands.

use git_spawn::command::config::ConfigScope;
use git_spawn::*;

fn args_of<C: GitCommand>(c: &C) -> Vec<String> {
    c.build_command_args()
}

#[test]
fn cherry_pick_commit_with_signoff() {
    let mut c = CherryPickCommand::new();
    c.commit("abc123").signoff().reference();
    assert_eq!(
        args_of(&c),
        vec!["cherry-pick", "--signoff", "-x", "abc123"]
    );
}

#[test]
fn cherry_pick_abort_short_circuits() {
    let mut c = CherryPickCommand::new();
    c.commit("abc").signoff().abort();
    assert_eq!(args_of(&c), vec!["cherry-pick", "--abort"]);
}

#[test]
fn grep_with_flags() {
    let mut c = GrepCommand::new("TODO");
    c.ignore_case().line_number().path("src");
    assert_eq!(
        args_of(&c),
        vec!["grep", "-i", "-n", "-e", "TODO", "--", "src"]
    );
}

#[test]
fn grep_in_tree() {
    let mut c = GrepCommand::new("fn main");
    c.fixed_strings().tree("HEAD");
    assert_eq!(args_of(&c), vec!["grep", "-F", "-e", "fn main", "HEAD"]);
}

#[test]
fn config_get_with_scope() {
    let c = ConfigCommand::get("user.email").scope(ConfigScope::Global);
    assert_eq!(args_of(&c), vec!["config", "--global", "user.email"]);
}

#[test]
fn config_set_local() {
    let c = ConfigCommand::set("user.name", "Ada").scope(ConfigScope::Local);
    assert_eq!(args_of(&c), vec!["config", "--local", "user.name", "Ada"]);
}

#[test]
fn config_list() {
    let c = ConfigCommand::list();
    assert_eq!(args_of(&c), vec!["config", "--list"]);
}

#[test]
fn reflog_show_head() {
    let c = ReflogCommand::show().max_count(5);
    assert_eq!(args_of(&c), vec!["reflog", "show", "-n5"]);
}

#[test]
fn worktree_add_with_branch() {
    let c = WorktreeCommand::add("/tmp/wt").new_branch("feature");
    assert_eq!(
        args_of(&c),
        vec!["worktree", "add", "-b", "feature", "/tmp/wt"]
    );
}

#[test]
fn worktree_list_porcelain() {
    let c = WorktreeCommand::list_porcelain();
    assert_eq!(args_of(&c), vec!["worktree", "list", "--porcelain"]);
}

#[test]
fn worktree_remove_force() {
    let c = WorktreeCommand::remove("/tmp/wt").force();
    assert_eq!(
        args_of(&c),
        vec!["worktree", "remove", "--force", "/tmp/wt"]
    );
}

#[test]
fn submodule_add_with_path() {
    let c = SubmoduleCommand::add("https://example.com/sub.git").path("vendor/sub");
    assert_eq!(
        args_of(&c),
        vec![
            "submodule",
            "add",
            "https://example.com/sub.git",
            "vendor/sub",
        ]
    );
}

#[test]
fn submodule_update_init_recursive() {
    let c = SubmoduleCommand::update().with_init().recursive();
    assert_eq!(
        args_of(&c),
        vec!["submodule", "update", "--init", "--recursive"]
    );
}

#[test]
fn bisect_start_with_bad_and_good() {
    let c = BisectCommand::start()
        .bad_commit("HEAD")
        .good_commit("v1.0");
    assert_eq!(args_of(&c), vec!["bisect", "start", "HEAD", "v1.0"]);
}

#[test]
fn bisect_bad_without_rev() {
    let c = BisectCommand::bad(None);
    assert_eq!(args_of(&c), vec!["bisect", "bad"]);
}

#[test]
fn bisect_run_command() {
    let c = BisectCommand::run(["cargo", "test"]);
    assert_eq!(args_of(&c), vec!["bisect", "run", "cargo", "test"]);
}
