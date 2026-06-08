# git-spawn

[![Crates.io](https://img.shields.io/crates/v/git-spawn.svg)](https://crates.io/crates/git-spawn)
[![Docs.rs](https://docs.rs/git-spawn/badge.svg)](https://docs.rs/git-spawn)
[![CI](https://github.com/joshrotenberg/git-spawn/actions/workflows/ci.yml/badge.svg)](https://github.com/joshrotenberg/git-spawn/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/git-spawn.svg)](#license)

An async Rust wrapper around the `git` CLI. Each git subcommand is a
builder-style struct; `.execute().await` spawns `git` as a subprocess and
returns typed output.

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
git-spawn = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

MSRV: **1.85** (Rust 2024 edition).

## Capabilities

- **Porcelain**: init, clone, add, commit, status, log, diff, show, branch,
  checkout, switch, merge, rebase, pull, push, fetch, remote, tag, stash,
  reset, restore, rm, mv
- **Plumbing**: rev-parse, ls-files, ls-tree, cat-file, hash-object,
  update-ref, for-each-ref
- **Advanced**: worktree, submodule, bisect, cherry-pick, grep, config, reflog
- **Typed parsers** (behind the `parse` feature, on by default) for
  `status --porcelain=v1 -z`, `log` with a fixed token format, and
  `diff --name-status -z`
- **Higher-level workflow helpers** (behind the `workflow` feature, off by
  default) — `repo.info()`, `repo.branches()`, `repo.tags()`,
  `repo.history()`, and `repo.workflow()` for one-call repo state, typed
  branch / tag / commit listings, and common compositions like
  `feature_branch`, `commit_all`, `sync`, `squash_merge`
- **Escape hatches** on every command (`.arg`, `.args`, `.flag`, `.option`)
  so flags the typed API hasn't surfaced are still reachable

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

The `parse` feature (on by default) also provides `parse_log` (paired with
`LOG_FORMAT`) and `parse_diff_name_status`. Enable the `serde` feature to get
`Serialize` / `Deserialize` on the parsed types.

### Workflow helpers (opt-in)

Enable the `workflow` feature for one-call repo state, typed listings, and
common compositions:

```toml
[dependencies]
git-spawn = { version = "0.1", features = ["workflow"] }
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

See the module docs for `info`, `branches`, `tags`, `history`, and `workflow`
for the full surface.

### Escape hatches

Every command supports `.arg`, `.args`, `.flag`, and `.option` for flags that
don't yet have a typed builder method:

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
| `parse`    |   on    | Typed parsers for status / log / diff output                           |
| `serde`    |   off   | `Serialize` / `Deserialize` on parsed types                            |
| `workflow` |   off   | Higher-level helpers: `info`, `branches`, `tags`, `history`, workflow compositions (implies `parse`) |

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
