# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/joshrotenberg/git-spawn/compare/v0.2.1...v0.3.0) - 2026-08-01


### Bug Fixes

- Publish dispatched release gates as commit statuses ([#148](https://github.com/joshrotenberg/git-spawn/pull/148))
- Terminate Git subprocess tree on timeout ([#143](https://github.com/joshrotenberg/git-spawn/pull/143))
- Kill the process group on timeout ([#89](https://github.com/joshrotenberg/git-spawn/pull/89))
- Emit -- separator before pathspecs in ls-tree and hash-object ([#58](https://github.com/joshrotenberg/git-spawn/pull/58))

### Documentation

- Perform a full README and rustdoc scrub before the next release ([#142](https://github.com/joshrotenberg/git-spawn/pull/142))

### Features

- Add Repository accessors for repo-scoped command builders ([#140](https://github.com/joshrotenberg/git-spawn/pull/140))
- Add inspection commands ([#139](https://github.com/joshrotenberg/git-spawn/pull/139))
- Add plumbing commands ([#138](https://github.com/joshrotenberg/git-spawn/pull/138))
- Support Git-global options before subcommands ([#136](https://github.com/joshrotenberg/git-spawn/pull/136))
- Preserve OS-native command arguments ([#135](https://github.com/joshrotenberg/git-spawn/pull/135))
- Expose raw nonzero command outcomes ([#134](https://github.com/joshrotenberg/git-spawn/pull/134))
- Support piped stdin bytes ([#133](https://github.com/joshrotenberg/git-spawn/pull/133))
- Add check-ignore raw command ([#125](https://github.com/joshrotenberg/git-spawn/pull/125))
- Add count-objects raw command ([#116](https://github.com/joshrotenberg/git-spawn/pull/116))
- Add version raw command ([#114](https://github.com/joshrotenberg/git-spawn/pull/114))
- Add var raw command ([#113](https://github.com/joshrotenberg/git-spawn/pull/113))
- Add name-rev raw command ([#112](https://github.com/joshrotenberg/git-spawn/pull/112))
- Add ls-remote raw command ([#110](https://github.com/joshrotenberg/git-spawn/pull/110))
- Add sparse-checkout raw command ([#109](https://github.com/joshrotenberg/git-spawn/pull/109))
- Add rerere raw command ([#108](https://github.com/joshrotenberg/git-spawn/pull/108))
- Add bundle raw command ([#107](https://github.com/joshrotenberg/git-spawn/pull/107))
- Add archive raw command ([#106](https://github.com/joshrotenberg/git-spawn/pull/106))
- Add maintenance raw command ([#105](https://github.com/joshrotenberg/git-spawn/pull/105))
- Add fsck raw command ([#104](https://github.com/joshrotenberg/git-spawn/pull/104))
- Add gc raw command ([#103](https://github.com/joshrotenberg/git-spawn/pull/103))
- Add shortlog raw command ([#102](https://github.com/joshrotenberg/git-spawn/pull/102))
- Add revert raw command ([#100](https://github.com/joshrotenberg/git-spawn/pull/100))
- Add clean raw command ([#99](https://github.com/joshrotenberg/git-spawn/pull/99))
- Add interpret-trailers raw command ([#98](https://github.com/joshrotenberg/git-spawn/pull/98))
- Add merge-base raw command ([#111](https://github.com/joshrotenberg/git-spawn/pull/111))
- Add range-diff raw command ([#97](https://github.com/joshrotenberg/git-spawn/pull/97))
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

- Audit and prune stale remote branches ([#149](https://github.com/joshrotenberg/git-spawn/pull/149))
- Bump amannn/action-semantic-pull-request from 5 to 6 ([#82](https://github.com/joshrotenberg/git-spawn/pull/82))
- Bump actions/checkout from 5 to 7 ([#81](https://github.com/joshrotenberg/git-spawn/pull/81))
- Add Dependabot config for cargo and github-actions ([#78](https://github.com/joshrotenberg/git-spawn/pull/78))

### Refactor

- Make command builders non-exhaustive before the next release ([#137](https://github.com/joshrotenberg/git-spawn/pull/137)) [**breaking**]

### Testing

- Wait for detached helper startup ([#146](https://github.com/joshrotenberg/git-spawn/pull/146))

### Changed

- Mark command builders and their public option/action enums as non-exhaustive.
  Commands must now be created through their documented constructors or
  `Repository` accessors instead of struct literals. Existing constructor and
  fluent-builder usage is unchanged. [**breaking**]

### Documentation

- Scrub the README and rustdoc for the next `0.3` release: inventory every
  command builder, typed parser and required output format, workflow helper,
  and Cargo feature; update installation snippets, requirements, platform
  limitations, examples, and the Git-library comparison.

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
