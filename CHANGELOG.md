# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
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
