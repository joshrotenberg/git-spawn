# git-spawn

[![Crates.io](https://img.shields.io/crates/v/git-spawn.svg)](https://crates.io/crates/git-spawn)
[![Docs.rs](https://docs.rs/git-spawn/badge.svg)](https://docs.rs/git-spawn)
[![CI](https://github.com/joshrotenberg/git-spawn/actions/workflows/ci.yml/badge.svg)](https://github.com/joshrotenberg/git-spawn/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/git-spawn.svg)](#license)

An async Rust wrapper around the `git` CLI. Each git subcommand is a
builder-style struct; `.execute().await` spawns `git` as a subprocess and
returns command-specific output (usually captured stdout/stderr, with typed
results where Git exposes a suitably stable shape).

```rust
use git_spawn::{GitCommand, Repository};

#[tokio::main]
async fn main() -> git_spawn::Result<()> {
    let repo = Repository::open("/path/to/repo")?;

    repo.add().all().execute().await?;
    repo.commit().message("snapshot").execute().await?;
    repo.push().remote("origin").refspec("main").execute().await?;

    Ok(())
}
```

## Install

```toml
[dependencies]
git-spawn = "0.3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

MSRV: **1.85** (Rust 2024 edition).

`git` must be installed and available on `PATH` at runtime. The crate supports
Unix and Windows. On Unix, a timed-out command's process group is terminated;
on other platforms only the direct child can be terminated portably. Hook
enable/disable helpers manipulate Unix executable bits and become existence
checks/no-ops, as documented, on platforms without those bits.

## Command builders

The crate currently wraps the following Git commands. Each name maps to a
builder in `git_spawn::command` (for example, `range-diff` maps to
`RangeDiffCommand`). Repository-dependent commands also have a pre-scoped
`Repository` accessor; standalone and hybrid cases are explained below.

| | Commands |
|-|-|
| A–C | `add`, `am`, `apply`, `archive`, `bisect`, `blame`, `branch`, `bundle`, `cat-file`, `check-attr`, `check-ignore`, `check-ref-format`, `checkout`, `cherry`, `cherry-pick`, `clean`, `clone`, `commit`, `commit-tree`, `config`, `count-objects` |
| D–L | `describe`, `diff`, `diff-files`, `diff-index`, `diff-tree`, `fetch`, `for-each-ref`, `format-patch`, `fsck`, `gc`, `grep`, `hash-object`, `init`, `interpret-trailers`, `log`, `ls-files`, `ls-remote`, `ls-tree` |
| M–R | `maintenance`, `merge`, `merge-base`, `merge-file`, `merge-tree`, `mktree`, `mv`, `name-rev`, `notes`, `pull`, `push`, `range-diff`, `read-tree`, `rebase`, `reflog`, `remote`, `rerere`, `reset`, `restore`, `rev-list`, `rev-parse`, `revert`, `rm` |
| S–W | `shortlog`, `show`, `show-ref`, `sparse-checkout`, `stash`, `status`, `submodule`, `switch`, `symbolic-ref`, `tag`, `update-index`, `update-ref`, `var`, `verify-commit`, `verify-tag`, `version`, `worktree`, `write-tree` |

Every builder has escape hatches (`.global_arg`, `.global_args`, `.arg`,
`.args`, `.flag`, and `.option`) so newly added or uncommon Git flags remain
reachable before the typed API exposes them.

## Choosing a git library for Rust

Three realistic options; pick by what you're building, not which is "best."

| Project       | What it is                                            | Needs `git` installed | Async-native | Honors local `git` config, hooks, credential helpers |
|---------------|-------------------------------------------------------|:---------------------:|:------------:|:----------------------------------------------------:|
| `git-spawn` | Async subprocess wrapper around the `git` CLI         |          yes          |     yes      |                         yes                          |
| `git2`        | Rust bindings to [libgit2](https://libgit2.org) (C)   |          no           |      no      |                       partial                        |
| `gix`         | Pure-Rust ([gitoxide](https://github.com/GitoxideLabs/gitoxide)) |          no           |     some     |                       partial                        |

- **`git-spawn`** -- automating workflows a human would script in bash (commit,
  push, rebase, cherry-pick, tagging) where behavior must match the user's real
  `git`: their `~/.gitconfig`, hooks, and credential helpers, run concurrently
  under `tokio`. Any flag the typed API hasn't surfaced is reachable via the
  escape hatches. Cost: a `git` binary on `PATH`, process-spawn overhead per
  call, and output parsing (or the `parse` feature).
- **`git2`** -- in-process object access (trees, blobs, commits) without
  requiring users to have `git` installed. Cost: a C dependency, no first-class
  async, and you wire up hooks/credentials yourself.
- **`gix`** -- a pure-Rust stack (no C toolchain, easy cross-compilation) with
  high-throughput object access for tooling built on git's data model. Cost: a
  still-evolving API on some write/network paths; like `git2`, doesn't run your
  hooks or credential helpers.

Rule of thumb: calling `git` *on behalf of a user* -> `git-spawn`; reading or
writing objects *without* a `git` install -> `git2` (mature, C) or `gix` (pure
Rust); building a merge engine or git server -> `gix`.

## Usage

### Repository handle

Command builders must be created through their documented constructors or a
`Repository` accessor and configured with fluent methods. Their fields remain
public for inspection and direct adjustment, but the structs are
non-exhaustive, so constructing them with struct literals outside this crate is
not supported. Command option and action enums are likewise non-exhaustive;
include a wildcard arm when matching them.

```rust
use git_spawn::{GitCommand, Repository};

async fn demo() -> git_spawn::Result<()> {
    // Open an existing repo (cheap, no process spawn).
    let repo = Repository::open("/path/to/repo")?;

    // Or initialize a new one.
    let fresh = Repository::init("/tmp/new-repo").await?;

    // Or clone.
    let cloned = Repository::clone(
        "https://github.com/octocat/Hello-World.git",
        "/tmp/hello",
    ).await?;

    Ok(())
}
```

`Repository` is cheap and cloneable; the accessor methods (`.add()`,
`.commit()`, `.log()`, ...) return commands pre-scoped to the repo's
working directory.

Accessors cover commands whose behavior depends on a repository or working
tree. Standalone commands use their direct constructors instead:
`VersionCommand` inspects the installed Git. Hybrid commands support both
forms: `repo.ls_remote()` reads configured remotes from that repository, while
`LsRemoteCommand::remote(...)` can query a standalone URL or path directly.
Similarly, full ref-name validation is standalone, while
`repo.check_ref_format(CheckRefFormatCommand::branch(...))` scopes branch-mode
reflog expansion such as `@{-1}`.

### Typed parsers

```rust
use git_spawn::{GitCommand, Repository};
use git_spawn::command::status::StatusFormat;
use git_spawn::parse::{parse_status, StatusKind};

async fn modified_paths() -> git_spawn::Result<()> {
    let repo = Repository::open("/path/to/repo")?;
    let out = repo.status()
        .format(StatusFormat::PorcelainV1)
        .null_terminate()
        .execute()
        .await?;

    for entry in parse_status(&out.stdout_str())? {
        if entry.worktree == StatusKind::Modified {
            println!("modified: {}", entry.path);
        }
    }
    Ok(())
}
```

The `parse` feature (on by default) provides the complete parser inventory
below. Pass the listed flags when a parser requires a machine-readable shape;
parsers marked “default” consume the command's ordinary output. Human-output
classifiers are necessarily sensitive to Git's locale and wording, and retain
raw text where useful.

| Parser | Required Git output |
|-|-|
| `parse_bisect` | default `git bisect` output (heuristic) |
| `parse_blame` | `git blame --porcelain` or `--line-porcelain` |
| `parse_cherry` | default `git cherry`; `-v` adds subjects |
| `parse_cherry_pick` | combined stdout/stderr from `git cherry-pick` (heuristic) |
| `parse_commit` | default `git commit` output |
| `parse_count_objects` / `parse_count_objects_terse` | `git count-objects -v` / default output; `--human-readable` is accepted |
| `parse_diff_name_status` | `git diff --name-status -z` |
| `parse_diff_numstat` / `parse_diff_stat` | `git diff --numstat -z` / `git diff --stat` |
| `parse_log` | `git log --format=<LOG_FORMAT>` |
| `parse_ls_remote` / `parse_ls_remote_symrefs` | default `git ls-remote` / `git ls-remote --symref` |
| `parse_ls_tree` / `parse_ls_tree_name_only` | default (optionally `-l`) / `git ls-tree --name-only` |
| `parse_merge` | default `git merge` stdout (heuristic) |
| `parse_notes_list` | `git notes list` |
| `parse_pull` / `parse_rebase` | default command output (heuristic) |
| `parse_reflog` | `git reflog show --format=<REFLOG_FORMAT>` |
| `parse_shortlog` | default `git shortlog` output |
| `parse_show` | `git show --format=<LOG_FORMAT>`; optionally `--stat` |
| `parse_status` / `parse_full_status` | `git status --porcelain=v1 -z` / the same plus `-b` |
| `parse_submodule_status` | `git submodule status` |
| `parse_version` | `git --version`; `--build-options` is accepted |

Enable `serde` to derive `Serialize` and `Deserialize` on parsed and workflow
value types.

### Checked and unchecked execution

`execute()` and `execute_raw()` are checked: any nonzero git exit becomes
`Error::CommandFailed`. Use `execute_raw_unchecked()` only when a git command
documents a nonzero status as ordinary control flow. It returns the captured
stdout, stderr, and exact exit status for every normally completed process;
spawn, I/O, and timeout failures remain errors.

```rust,no_run
use git_spawn::{GitCommand, Repository};

async fn has_changes() -> git_spawn::Result<bool> {
    let repo = Repository::open("/path/to/repo")?;
    let output = repo
        .diff()
        .args(["--quiet", "--exit-code"])
        .execute_raw_unchecked()
        .await?;

    Ok(output.exit_code == 1)
}
```

### Workflow helpers (opt-in)

Enable the `workflow` feature for one-call repo state, typed listings, and
common compositions:

```toml
[dependencies]
git-spawn = { version = "0.3", features = ["workflow"] }
```

```rust
use git_spawn::Repository;

async fn quick_status() -> git_spawn::Result<()> {
    let repo = Repository::open("/repo")?;

    let info = repo.info().await?;
    println!("{} (dirty: {}, ahead {} / behind {})",
        info.branch.as_deref().unwrap_or("(detached)"),
        info.dirty, info.ahead, info.behind);

    for b in repo.branches().list().await? {
        println!("  {}{}", if b.current { "* " } else { "  " }, b.name);
    }

    for c in repo.history().max_count(5).execute().await? {
        println!("  {} {}", c.short_sha, c.subject);
    }

    // Multi-step shortcuts.
    repo.workflow().feature_branch("feature/x", "main").await?;
    repo.workflow().commit_all("wip").await?;
    Ok(())
}
```

The complete helper surface is:

| Accessor | Operations |
|-|-|
| `info()` | repository, branch, upstream, default-branch, dirty, and ahead/behind summary |
| `branches()` | `list`, `list_matching`, `delete_merged`, `rename` |
| `changes()` | typed staged, unstaged, untracked, and tracking `summary` |
| `conflicts()` | `list`, `resolve` |
| `history()` | filtered commit walk: `max_count`, `skip`, `since`, `until`, `author`, `grep`, `revision`, `path`, `reverse`, `execute` |
| `hooks()` | `list`, `install`, `remove`, `enable`, `disable` |
| `patches()` | `format` (with `output_dir`, `numbered`, `signoff`), `apply`, `am` |
| `remotes()` | `list`, `add`, `remove`, `rename`, `set_url`, `get_url` |
| `search()` | `pattern`, `in_path`, `in_paths`, `case_insensitive`, `word_regexp`, `fixed_strings`, `extended_regexp`, `perl_regexp`, `cached`, `execute` |
| `signing()` | `signing_key`, `format`, `sign_commits`, `sign_tags`, `config`, and the corresponding `set_*` methods |
| `stashes()` | `list`, `push`, `pop`, `apply`, `drop`, `clear` |
| `tags()` | `list`, `list_matching`, `create`, `create_annotated`, `delete` |
| `workflow()` | `feature_branch`, `commit_all`, `sync`, `squash_merge` |

See each module's rustdoc for return types, exact behavior, and limitations.

### Escape hatches

Every command supports `.global_arg` and `.global_args` for Git-global options
that must precede the subcommand. The existing `.arg`, `.args`, `.flag`, and
`.option` methods append arguments after the typed subcommand arguments:

```rust
use git_spawn::{GitCommand, Repository};

async fn shortstat() -> git_spawn::Result<()> {
    let repo = Repository::open("/repo")?;
    // `--shortstat` isn't a typed method on DiffCommand, but this still works:
    let out = repo.diff().cached().arg("--shortstat").execute().await?;
    println!("{}", out.stdout_str());
    Ok(())
}
```

For example, invocation-local configuration and a repository path can be
provided without changing the process working directory or persistent config:

```rust,no_run
use git_spawn::{GitCommand, StatusCommand};

# async fn status() -> git_spawn::Result<()> {
let mut command = StatusCommand::new();
let out = command
    .global_args(["-c", "core.hooksPath=/dev/null"])
    .global_args(["--no-optional-locks", "-C", "/repo"])
    .execute()
    .await?;
# Ok(())
# }
```

### Timeouts, env, working dir

```rust
use std::time::Duration;
use git_spawn::{GitCommand, Repository};

async fn careful_fetch() -> git_spawn::Result<()> {
    let repo = Repository::open("/repo")?;
    let mut cmd = repo.fetch();
    cmd.remote("origin")
        .with_timeout(Duration::from_secs(30))
        .env("GIT_TERMINAL_PROMPT", "0");
    cmd.execute().await?;
    Ok(())
}
```

## Feature flags

| Flag       | Default | Purpose                                                                |
|------------|:-------:|------------------------------------------------------------------------|
| `parse`    |   on    | All typed output parsers listed above                                  |
| `serde`    |   off   | `Serialize` / `Deserialize` derives on parsed and workflow value types |
| `workflow` |   off   | All higher-level helpers listed above; implies `parse`                  |

## Contributing

PRs welcome. Please run before submitting:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --all-features --no-deps
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.
