# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
## [0.3.0](https://github.com/joshrotenberg/git-spawn/compare/v0.2.1...v0.3.0) - 2026-07-23


### Bug Fixes

- Kill the process group on timeout ([#89](https://github.com/joshrotenberg/git-spawn/pull/89))
- Emit -- separator before pathspecs in ls-tree and hash-object ([#58](https://github.com/joshrotenberg/git-spawn/pull/58))

### Features

- Add blame raw command ([#101](https://github.com/joshrotenberg/git-spawn/pull/101))
- Add cherry raw command ([#96](https://github.com/joshrotenberg/git-spawn/pull/96))
- Add verify-commit and verify-tag raw commands ([#95](https://github.com/joshrotenberg/git-spawn/pull/95))
- Add patches helper ([#94](https://github.com/joshrotenberg/git-spawn/pull/94))
- Add AmCommand raw command ([#93](https://github.com/joshrotenberg/git-spawn/pull/93))
- Add ApplyCommand raw command ([#92](https://github.com/joshrotenberg/git-spawn/pull/92))
- Add FormatPatchCommand raw command ([#91](https://github.com/joshrotenberg/git-spawn/pull/91))
- Add cat_file, hash_object, for_each_ref, update_ref accessors ([#90](https://github.com/joshrotenberg/git-spawn/pull/90))
- Add typed hooks helper ([#80](https://github.com/joshrotenberg/git-spawn/pull/80))
- Add typed search helper ([#79](https://github.com/joshrotenberg/git-spawn/pull/79))
- Add changes helper ([#77](https://github.com/joshrotenberg/git-spawn/pull/77))
- Add typed signing helper ([#75](https://github.com/joshrotenberg/git-spawn/pull/75))
- Add typed remotes helper ([#74](https://github.com/joshrotenberg/git-spawn/pull/74))
- Add typed conflicts helper ([#73](https://github.com/joshrotenberg/git-spawn/pull/73))
- Add typed stashes helper ([#72](https://github.com/joshrotenberg/git-spawn/pull/72))
- Add typed BisectResult output for git bisect ([#67](https://github.com/joshrotenberg/git-spawn/pull/67))
- Add branch/ahead/behind metadata to typed status output (WIP) ([#70](https://github.com/joshrotenberg/git-spawn/pull/70))
- Add stat/numstat/raw diff parsers and a typed Diff aggregate (WIP) ([#71](https://github.com/joshrotenberg/git-spawn/pull/71))
- Add typed SubmoduleStatus output for submodule command (WIP) ([#69](https://github.com/joshrotenberg/git-spawn/pull/69))
- Add typed RebaseResult output for git rebase ([#66](https://github.com/joshrotenberg/git-spawn/pull/66))
- Add typed ShowResult output for show ([#68](https://github.com/joshrotenberg/git-spawn/pull/68))
- Add typed CherryPickResult output for cherry-pick ([#65](https://github.com/joshrotenberg/git-spawn/pull/65))
- Add typed TreeEntry output for ls-tree ([#64](https://github.com/joshrotenberg/git-spawn/pull/64))
- Add typed ReflogEntry output for reflog show ([#63](https://github.com/joshrotenberg/git-spawn/pull/63))
- Add typed CommitResult output for commit ([#61](https://github.com/joshrotenberg/git-spawn/pull/61))
- Add typed MergeResult output for git merge ([#62](https://github.com/joshrotenberg/git-spawn/pull/62))
- Add typed PullResult output for git pull ([#60](https://github.com/joshrotenberg/git-spawn/pull/60))

### Miscellaneous

- Bump amannn/action-semantic-pull-request from 5 to 6 ([#82](https://github.com/joshrotenberg/git-spawn/pull/82))
- Bump actions/checkout from 5 to 7 ([#81](https://github.com/joshrotenberg/git-spawn/pull/81))
- Add Dependabot config for cargo and github-actions ([#78](https://github.com/joshrotenberg/git-spawn/pull/78))

## [0.2.1](https://github.com/joshrotenberg/git-spawn/compare/v0.2.0...v0.2.1) - 2026-06-08


### Features

- Add generic git notes command wrapper (closes #31) ([#32](https://github.com/joshrotenberg/git-spawn/pull/32))

### Miscellaneous

- Backfill 0.2.0 changelog and guard release automation ([#29](https://github.com/joshrotenberg/git-spawn/pull/29))

## [0.2.0](https://github.com/joshrotenberg/git-spawn/compare/v0.1.0...v0.2.0) - 2026-06-08

### Features

- Add `Repository` accessors for plumbing commands: `rev_parse`, `describe`, `ls_files`, `ls_tree`, `show_ref`, `symbolic_ref` ([#22](https://github.com/joshrotenberg/git-spawn/issues/22))
- Add no-match-tolerant execution: `GrepCommand::execute_allow_no_match` and `ConfigCommand::execute_value_opt` return `Ok(None)` for the exit-1 (no match / missing key) case instead of `CommandFailed` ([#21](https://github.com/joshrotenberg/git-spawn/issues/21))

### Bug Fixes

- `CommandOutput.stdout` is now `Vec<u8>` instead of `String`; binary output (e.g. `cat-file` on a blob) is no longer corrupted by lossy UTF-8 decoding. Read it via `stdout_str()` / `stdout_bytes()`, and use `CatFileCommand::execute_bytes()` for raw blob bytes ([#23](https://github.com/joshrotenberg/git-spawn/issues/23)) [**breaking**]
- `branch` force-delete emits `-D <name>` instead of the invalid `-D -d <name>` ([#25](https://github.com/joshrotenberg/git-spawn/issues/25))
- `parse_status` reads the original-path field only when the index column is a rename/copy, matching real porcelain v1 `-z` output ([#20](https://github.com/joshrotenberg/git-spawn/issues/20))

### Documentation

- Trim git library comparison section ([#18](https://github.com/joshrotenberg/git-spawn/pull/18))

### Refactor

- Unify builder modifier style to `&mut Self` across the action-enum commands (stash, config, reflog, bisect, symbolic_ref, worktree, submodule) ([#19](https://github.com/joshrotenberg/git-spawn/issues/19)) [**breaking**]
- Consolidate shared integration-test helpers into `tests/common` ([#24](https://github.com/joshrotenberg/git-spawn/issues/24))

### Miscellaneous

- Release v0.1.0 ([#16](https://github.com/joshrotenberg/git-spawn/pull/16))

## [0.1.0](https://github.com/joshrotenberg/git-spawn/releases/tag/v0.1.0) - 2026-05-22


### Documentation

- Add README with usage, comparison to git2/gix, and dual license files

### Features

- Add tags, history, and workflow modules to workflow feature
- Add workflow feature with info and branches modules
- Add runnable examples and three small plumbing commands
- Add advanced commands (worktree, submodule, bisect, cherry-pick, grep, config, reflog)
- Add plumbing commands, typed parsers, and expanded rustdoc
- Add 23 porcelain command wrappers and Repository ergonomics
- Initial scaffold with error, command executor, and repository handle

### Miscellaneous

- Rename crate to git-spawn ([#15](https://github.com/joshrotenberg/git-spawn/pull/15))
- Appease clippy 1.95 unnecessary_sort_by lint
