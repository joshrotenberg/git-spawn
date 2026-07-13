//! Integration tests for advanced (Phase 4) commands.

use git_spawn::command::config::ConfigScope;
use git_spawn::{
    BisectCommand, CommandExecutor, ConfigCommand, GitCommand, ReflogCommand, Repository,
    SubmoduleCommand, WorktreeCommand,
};

mod common;
use common::configure_identity;

async fn seed_repo() -> (tempfile::TempDir, Repository) {
    let (tmp, repo) = common::init_repo().await;
    std::fs::write(repo.path().join("a.txt"), "one\n").unwrap();
    repo.add().path("a.txt").execute().await.unwrap();
    repo.commit().message("c1").execute().await.unwrap();
    (tmp, repo)
}

#[tokio::test]
async fn config_set_and_get() {
    let (_tmp, repo) = seed_repo().await;
    repo.config(ConfigCommand::set("test.key", "hello"))
        .scope(ConfigScope::Local)
        .execute()
        .await
        .unwrap();

    let value = repo
        .config(ConfigCommand::get("test.key"))
        .scope(ConfigScope::Local)
        .execute_value()
        .await
        .unwrap();
    assert_eq!(value, "hello");
}

#[tokio::test]
async fn grep_matches_content() {
    let (_tmp, repo) = seed_repo().await;
    std::fs::write(repo.path().join("greeting.txt"), "hello world\n").unwrap();
    repo.add().path("greeting.txt").execute().await.unwrap();
    repo.commit().message("greet").execute().await.unwrap();

    let out = repo
        .grep("hello")
        .fixed_strings()
        .line_number()
        .execute()
        .await
        .unwrap();
    assert!(out.stdout_str().contains("greeting.txt"));
    assert!(out.stdout_str().contains("hello world"));
}

#[tokio::test]
async fn grep_no_match_errors() {
    // git grep exits 1 when nothing matches.
    let (_tmp, repo) = seed_repo().await;
    let err = repo
        .grep("nothing-matches-this-xyz")
        .fixed_strings()
        .execute()
        .await
        .unwrap_err();
    assert!(matches!(err, git_spawn::Error::CommandFailed { .. }));
}

#[tokio::test]
async fn grep_no_match_ok_returns_none() {
    let (_tmp, repo) = seed_repo().await;
    std::fs::write(repo.path().join("greeting.txt"), "hello world\n").unwrap();
    repo.add().path("greeting.txt").execute().await.unwrap();
    repo.commit().message("greet").execute().await.unwrap();

    // No match -> Ok(None), not CommandFailed.
    let none = repo
        .grep("nothing-matches-this-xyz")
        .fixed_strings()
        .execute_allow_no_match()
        .await
        .unwrap();
    assert!(none.is_none());

    // A match -> Ok(Some(output)).
    let some = repo
        .grep("hello")
        .fixed_strings()
        .execute_allow_no_match()
        .await
        .unwrap()
        .expect("expected a match");
    assert!(some.stdout_str().contains("greeting.txt"));
}

#[tokio::test]
async fn config_missing_key_opt_returns_none() {
    let (_tmp, repo) = seed_repo().await;

    // Missing key -> Ok(None).
    let missing = repo
        .config(ConfigCommand::get("nope.absent"))
        .scope(ConfigScope::Local)
        .execute_value_opt()
        .await
        .unwrap();
    assert!(missing.is_none());

    // Present key -> Ok(Some(value)).
    repo.config(ConfigCommand::set("present.key", "yes"))
        .scope(ConfigScope::Local)
        .execute()
        .await
        .unwrap();
    let present = repo
        .config(ConfigCommand::get("present.key"))
        .scope(ConfigScope::Local)
        .execute_value_opt()
        .await
        .unwrap();
    assert_eq!(present.as_deref(), Some("yes"));
}

#[tokio::test]
async fn reflog_shows_initial_commit() {
    let (_tmp, repo) = seed_repo().await;
    let out = repo
        .reflog(ReflogCommand::show())
        .max_count(10)
        .execute()
        .await
        .unwrap();
    assert!(out.stdout_str().contains("c1"));
}

#[tokio::test]
async fn reflog_show_parses_typed_entries() {
    use git_spawn::parse::{REFLOG_FORMAT, parse_reflog};

    let (_tmp, repo) = seed_repo().await;
    std::fs::write(repo.path().join("b.txt"), "two\n").unwrap();
    repo.add().path("b.txt").execute().await.unwrap();
    repo.commit().message("c2").execute().await.unwrap();

    let out = repo
        .reflog(ReflogCommand::show())
        .format(REFLOG_FORMAT)
        .execute()
        .await
        .unwrap();

    let entries = parse_reflog(&out.stdout_str()).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].selector, "HEAD@{0}");
    assert_eq!(entries[0].action, "commit");
    assert_eq!(entries[0].message, "c2");
    assert_eq!(entries[1].selector, "HEAD@{1}");
    assert!(!entries[0].hash.is_empty());
    assert!(!entries[0].abbreviated_hash.is_empty());
}

#[tokio::test]
async fn cherry_pick_brings_commit_forward() {
    let (_tmp, repo) = seed_repo().await;

    // Branch topic, add a second commit, then cherry-pick it back onto main.
    repo.switch().create("topic").execute().await.unwrap();
    std::fs::write(repo.path().join("b.txt"), "two\n").unwrap();
    repo.add().path("b.txt").execute().await.unwrap();
    repo.commit().message("add-b").execute().await.unwrap();

    let topic_head = {
        let mut rp = git_spawn::RevParseCommand::new();
        rp.current_dir(repo.path()).arg_str("HEAD");
        rp.execute().await.unwrap()
    };

    repo.switch().target("main").execute().await.unwrap();
    repo.cherry_pick()
        .commit(&topic_head)
        .execute()
        .await
        .unwrap();
    assert!(repo.path().join("b.txt").exists());
}

#[tokio::test]
async fn worktree_add_and_list_and_remove() {
    let tmp = tempfile::tempdir().unwrap();
    let (_tmp, repo) = {
        let path = tmp.path().join("main-repo");
        std::fs::create_dir_all(&path).unwrap();
        let mut init = git_spawn::InitCommand::in_directory(&path);
        init.initial_branch("main").quiet();
        let r = init.execute().await.unwrap();
        configure_identity(&r);
        std::fs::write(r.path().join("x"), "x").unwrap();
        r.add().path("x").execute().await.unwrap();
        r.commit().message("init").execute().await.unwrap();
        (tmp, r)
    };

    let wt_path = repo.path().parent().unwrap().join("wt");
    repo.worktree(WorktreeCommand::add(&wt_path))
        .new_branch("wt-branch")
        .execute()
        .await
        .unwrap();
    assert!(wt_path.join("x").exists());

    let list = repo
        .worktree(WorktreeCommand::list_porcelain())
        .execute()
        .await
        .unwrap();
    assert!(list.stdout_str().contains("worktree "));

    repo.worktree(WorktreeCommand::remove(&wt_path))
        .force()
        .execute()
        .await
        .unwrap();
    assert!(!wt_path.exists());
}

#[tokio::test]
async fn submodule_status_on_empty_repo() {
    // `submodule status` on a repo without submodules returns empty stdout
    // with exit 0. Just confirm it doesn't crash.
    let (_tmp, repo) = seed_repo().await;
    let out = repo
        .submodule(SubmoduleCommand::status())
        .execute()
        .await
        .unwrap();
    assert!(out.stdout_str().trim().is_empty());
}

#[tokio::test]
async fn bisect_start_and_reset() {
    let (_tmp, repo) = seed_repo().await;
    // Create a second commit so bisect has somewhere to walk.
    std::fs::write(repo.path().join("a.txt"), "two\n").unwrap();
    repo.add().path("a.txt").execute().await.unwrap();
    repo.commit().message("c2").execute().await.unwrap();

    // Start bisect with explicit bad=HEAD, good=HEAD~1.
    repo.bisect(BisectCommand::start())
        .bad_commit("HEAD")
        .good_commit("HEAD~1")
        .execute()
        .await
        .unwrap();

    // Reset cleans up the bisect state.
    repo.bisect(BisectCommand::reset(None))
        .execute()
        .await
        .unwrap();
}

#[tokio::test]
async fn bisect_converges_on_first_bad_commit() {
    use git_spawn::parse::BisectStatus;

    let (_tmp, repo) = seed_repo().await;

    // Seed a short history where c3 is the first bad commit.
    std::fs::write(repo.path().join("a.txt"), "two\n").unwrap();
    repo.add().path("a.txt").execute().await.unwrap();
    repo.commit().message("c2").execute().await.unwrap();

    std::fs::write(repo.path().join("a.txt"), "three\n").unwrap();
    repo.add().path("a.txt").execute().await.unwrap();
    repo.commit().message("c3").execute().await.unwrap();

    let bad_sha = {
        let mut rp = git_spawn::RevParseCommand::new();
        rp.current_dir(repo.path()).arg_str("HEAD");
        rp.execute().await.unwrap()
    };

    std::fs::write(repo.path().join("a.txt"), "four\n").unwrap();
    repo.add().path("a.txt").execute().await.unwrap();
    repo.commit().message("c4").execute().await.unwrap();

    repo.bisect(BisectCommand::start())
        .bad_commit("HEAD")
        .good_commit("HEAD~3")
        .execute()
        .await
        .unwrap();

    // Narrow down: HEAD~3 is c1 (good), HEAD is c4 (bad). Bisect checks out
    // c2 or c3 next; mark it bad or good based on whether it's before or at
    // the seeded bad commit, until the session converges.
    let mut found: Option<git_spawn::parse::BisectResult> = None;
    for _ in 0..10 {
        let current = {
            let mut rp = git_spawn::RevParseCommand::new();
            rp.current_dir(repo.path()).arg_str("HEAD");
            rp.execute().await.unwrap()
        };
        let mark = if current == bad_sha {
            BisectCommand::bad(None)
        } else {
            // Determine order via merge-base --is-ancestor: current is good
            // if it's an ancestor of (or equal to) the seeded bad commit.
            //
            // Spawned through the crate's tokio-based executor rather than
            // `std::process::Command`: mixing blocking std child processes
            // with tokio's async SIGCHLD-driven reaper in the same runtime
            // races on Unix (tokio's wildcard `waitpid` can reap the std
            // child first), which showed up as spurious non-convergence on
            // loaded macOS CI runners.
            let is_ancestor = CommandExecutor::new()
                .cwd(repo.path())
                .execute_command(vec![
                    "merge-base".to_string(),
                    "--is-ancestor".to_string(),
                    current.clone(),
                    bad_sha.clone(),
                ])
                .await
                .is_ok();
            if is_ancestor {
                BisectCommand::good(vec![])
            } else {
                BisectCommand::bad(None)
            }
        };
        let cmd = repo.bisect(mark);
        let output = cmd.execute().await.unwrap();
        let result = cmd.parse_result(&output).unwrap();
        if result.status == BisectStatus::Found {
            found = Some(result);
            break;
        }
    }

    let result = found.expect("bisect should converge within 10 steps");
    assert_eq!(result.status, BisectStatus::Found);
    assert_eq!(result.bad_commit.as_deref(), Some(bad_sha.as_str()));

    repo.bisect(BisectCommand::reset(None))
        .execute()
        .await
        .unwrap();
}
