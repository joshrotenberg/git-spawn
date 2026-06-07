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

This crate is one of three realistic options. Pick based on what you're
building, not which is "best."

| Project       | What it is                                            | You need `git` installed | Async-native | Honors local `git` config, hooks, credential helpers |
|---------------|-------------------------------------------------------|:------------------------:|:------------:|:----------------------------------------------------:|
| `git-spawn` | Async subprocess wrapper around the `git` CLI         |           yes            |     yes      |                         yes                          |
| `git2`        | Rust bindings to [libgit2](https://libgit2.org) (C)   |            no            |      no      |                       partial                        |
| `gix`         | Pure-Rust ([gitoxide](https://github.com/GitoxideLabs/gitoxide)) |            no            |     some     |                       partial                        |

### When to reach for `git-spawn`

- You're automating **workflows a human would script in bash**: commits,
  pushes, rebases, cherry-picks, worktree setup, release tagging.
- You want behavior to match **exactly what the user's `git` does** on the
  host, including `~/.gitconfig`, `core.*` settings, pre-commit hooks,
  SSH/HTTPS credential helpers, and `safe.directory`.
- You're already in a `tokio` program and want to run several git operations
  concurrently (fetching multiple remotes, building repos in parallel).
- You need a feature that libgit2 / gix haven't implemented yet. Any `git`
  flag works via the escape hatches.

Trade-offs:

- A `git` binary must be on `PATH` at runtime.
- Each operation has process-spawn overhead (low hundreds of microseconds
  to a few milliseconds). Fine for workflow automation; not fine for
  tight loops over thousands of objects.
- Output parsing is on you (or the `parse` feature).

### When to reach for `git2`

- You need **in-process object database access** (walking trees, reading
  blobs, creating commits) without spawning a subprocess per call.
- You're building a tool that should work without requiring users to have
  `git` installed (GUIs, IDE plugins, CI containers).
- You're comfortable with a C dependency: `git2` links libgit2, which
  means a C compiler / CMake (or the vendored build) at build time.
- Your program is sync or you're OK running libgit2 calls on a blocking
  thread pool.

Trade-offs:

- No first-class async. You'll use `spawn_blocking` if you're in tokio.
- libgit2 doesn't invoke the user's `git` hooks or credential helpers by
  default; you implement credential callbacks yourself.
- Some newer git features lag behind the CLI (partial clone variants,
  SHA-256, sparse checkout modes).

### When to reach for `gix`

- You want a **pure-Rust** stack: no C toolchain, deterministic builds,
  easy cross-compilation, no libgit2 CVEs to track.
- You need high-throughput object access or want to build sophisticated
  tooling on top of git's data model. `gix` is split into many focused
  crates so you can take only what you need.
- You're willing to accept a **still-evolving API** in some areas. The
  read paths are solid; some write/network paths are newer.

Trade-offs:

- Like `git2`, doesn't run `git` hooks or credential helpers for you.
- Not every git feature is implemented yet; check the gitoxide project
  board for status of what you need.

### Quick decision guide

- "I'm calling `git push` / `git rebase` / `git clone` on behalf of a user."
  -> `git-spawn`.
- "I'm walking commit history to generate a report, or reading blobs, and
  I can't require `git` to be installed."
  -> `git2` if you need maturity and don't mind C; `gix` if you want pure Rust.
- "I'm building a merge engine, a git server, or a CAS-backed fetcher."
  -> `gix`.
- "I want to let a user pick a commit and cherry-pick it onto another
  branch, respecting their hooks."
  -> `git-spawn`.

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
