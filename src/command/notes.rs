//! `git notes` — add or inspect object notes stored in a `refs/notes/*` namespace.
//!
//! Notes attach mutable metadata to any git object (commit, blob, tree, tag)
//! without rewriting it. Each note lives in a ref namespace — the default is
//! `refs/notes/commits`, but any namespace works. This wrapper keeps the
//! namespace raw: pass whatever [`ref_namespace`](NotesCommand::ref_namespace)
//! value you want (`refs/notes/embeddings`, a short `build`, ...) and git-spawn
//! forwards it verbatim via `--ref` without prepending `refs/notes/`.
//!
//! Note payloads can be binary or large (the underlying object is just a blob).
//! Prefer [`message_file`](NotesCommand::message_file) over
//! [`message`](NotesCommand::message) for such payloads: it reads the bytes from
//! a file (`-F`), dodging argument-length limits, and pairs naturally with
//! [`no_stripspace`](NotesCommand::no_stripspace) for byte-exact round-trips.
//!
//! # Sharing notes across repositories
//!
//! Notes refs are not fetched or pushed by default. Because
//! [`PushCommand`](crate::command::push::PushCommand) and
//! [`FetchCommand`](crate::command::fetch::FetchCommand) accept arbitrary
//! refspecs, moving a notes namespace between repositories needs no dedicated
//! method — just name the notes ref:
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::{GitCommand, Repository};
//! use git_spawn::command::notes::NotesCommand;
//!
//! let repo = Repository::open("/repo")?;
//!
//! // Attach a note to HEAD in a custom namespace.
//! repo.notes(NotesCommand::add())
//!     .ref_namespace("refs/notes/test")
//!     .object("HEAD")
//!     .message("reviewed")
//!     .execute()
//!     .await?;
//!
//! // Publish the namespace to a remote, then fetch it back elsewhere.
//! repo.push()
//!     .remote("origin")
//!     .refspec("refs/notes/test:refs/notes/test")
//!     .execute()
//!     .await?;
//! repo.fetch()
//!     .remote("origin")
//!     .refspec("refs/notes/test:refs/notes/test")
//!     .execute()
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Actions supported by `git notes`.
#[derive(Debug, Clone)]
pub enum NotesAction {
    /// `git notes add [-f] [--allow-empty] [-m <msg> | -F <file>] [<object>]`.
    Add {
        /// Target object (defaults to `HEAD` when `None`).
        object: Option<String>,
        /// `-m <msg>` note contents as a string.
        message: Option<String>,
        /// `-F <file>` note contents read from a file.
        message_file: Option<PathBuf>,
        /// `-f` replace an existing note.
        force: bool,
        /// `--allow-empty` store a note even if it is empty.
        allow_empty: bool,
        /// `--no-stripspace` keep the payload byte-for-byte.
        no_stripspace: bool,
    },
    /// `git notes append [--allow-empty] [-m <msg> | -F <file>] [<object>]`.
    Append {
        /// Target object (defaults to `HEAD` when `None`).
        object: Option<String>,
        /// `-m <msg>` text to append.
        message: Option<String>,
        /// `-F <file>` payload to append, read from a file.
        message_file: Option<PathBuf>,
        /// `--allow-empty` store a note even if it is empty.
        allow_empty: bool,
        /// `--no-stripspace` keep the payload byte-for-byte.
        no_stripspace: bool,
    },
    /// `git notes copy [-f] <from-object> <to-object>`.
    Copy {
        /// Object to copy the note from.
        from: String,
        /// Object to copy the note to.
        to: String,
        /// `-f` overwrite an existing note on the target.
        force: bool,
    },
    /// `git notes show [<object>]`.
    ///
    /// Exits non-zero when no note exists for the object — surfaced as a clean
    /// [`Error::CommandFailed`](crate::error::Error::CommandFailed), never
    /// swallowed.
    Show {
        /// Target object (defaults to `HEAD` when `None`).
        object: Option<String>,
    },
    /// `git notes list [<object>]`.
    List {
        /// Restrict to a single object (otherwise lists every note).
        object: Option<String>,
    },
    /// `git notes remove [--ignore-missing] [<object>]`.
    Remove {
        /// Target object (defaults to `HEAD` when `None`).
        object: Option<String>,
        /// `--ignore-missing` exit zero when there is no note to remove.
        ignore_missing: bool,
    },
    /// `git notes prune [-n] [-v]` — drop notes for non-existent objects.
    Prune {
        /// `-n` dry run: report without removing.
        dry_run: bool,
        /// `-v` report pruned objects.
        verbose: bool,
    },
}

/// Builder for `git notes`.
#[derive(Debug, Clone)]
pub struct NotesCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// `--ref <namespace>` — the notes ref to operate on (raw, not prefixed).
    pub ref_namespace: Option<String>,
    /// Action.
    pub action: NotesAction,
}

impl NotesCommand {
    /// `notes add`.
    #[must_use]
    pub fn add() -> Self {
        Self {
            executor: CommandExecutor::default(),
            ref_namespace: None,
            action: NotesAction::Add {
                object: None,
                message: None,
                message_file: None,
                force: false,
                allow_empty: false,
                no_stripspace: false,
            },
        }
    }

    /// `notes append`.
    #[must_use]
    pub fn append() -> Self {
        Self {
            executor: CommandExecutor::default(),
            ref_namespace: None,
            action: NotesAction::Append {
                object: None,
                message: None,
                message_file: None,
                allow_empty: false,
                no_stripspace: false,
            },
        }
    }

    /// `notes copy <from> <to>`.
    pub fn copy(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            ref_namespace: None,
            action: NotesAction::Copy {
                from: from.into(),
                to: to.into(),
                force: false,
            },
        }
    }

    /// `notes show`.
    #[must_use]
    pub fn show() -> Self {
        Self {
            executor: CommandExecutor::default(),
            ref_namespace: None,
            action: NotesAction::Show { object: None },
        }
    }

    /// `notes list`.
    #[must_use]
    pub fn list() -> Self {
        Self {
            executor: CommandExecutor::default(),
            ref_namespace: None,
            action: NotesAction::List { object: None },
        }
    }

    /// `notes remove`.
    #[must_use]
    pub fn remove() -> Self {
        Self {
            executor: CommandExecutor::default(),
            ref_namespace: None,
            action: NotesAction::Remove {
                object: None,
                ignore_missing: false,
            },
        }
    }

    /// `notes prune`.
    #[must_use]
    pub fn prune() -> Self {
        Self {
            executor: CommandExecutor::default(),
            ref_namespace: None,
            action: NotesAction::Prune {
                dry_run: false,
                verbose: false,
            },
        }
    }

    /// Operate on the given notes ref namespace, emitted as `--ref <ns>`.
    ///
    /// The value is forwarded verbatim — git-spawn does not prepend
    /// `refs/notes/`. Pass a full ref (`refs/notes/embeddings`) or a short name
    /// (`build`); git resolves short names under `refs/notes/` itself.
    pub fn ref_namespace(&mut self, ns: impl Into<String>) -> &mut Self {
        self.ref_namespace = Some(ns.into());
        self
    }

    /// Set the target object (for `add`, `append`, `show`, `list`, `remove`).
    pub fn object(&mut self, o: impl Into<String>) -> &mut Self {
        let o = o.into();
        match &mut self.action {
            NotesAction::Add { object, .. }
            | NotesAction::Append { object, .. }
            | NotesAction::Show { object, .. }
            | NotesAction::List { object, .. }
            | NotesAction::Remove { object, .. } => *object = Some(o),
            NotesAction::Copy { .. } | NotesAction::Prune { .. } => {}
        }
        self
    }

    /// Note payload as an inline string, emitted as `-m <msg>` (for `add` /
    /// `append`).
    ///
    /// For binary or large payloads prefer [`message_file`](Self::message_file)
    /// to avoid argument-length limits.
    pub fn message(&mut self, m: impl Into<String>) -> &mut Self {
        let m = m.into();
        match &mut self.action {
            NotesAction::Add { message, .. } | NotesAction::Append { message, .. } => {
                *message = Some(m);
            }
            _ => {}
        }
        self
    }

    /// Note payload read from a file, emitted as `-F <path>` (for `add` /
    /// `append`). The preferred form for binary or multi-kilobyte payloads.
    pub fn message_file(&mut self, p: impl Into<PathBuf>) -> &mut Self {
        let p = p.into();
        match &mut self.action {
            NotesAction::Add { message_file, .. } | NotesAction::Append { message_file, .. } => {
                *message_file = Some(p);
            }
            _ => {}
        }
        self
    }

    /// `-f` — replace an existing note (for `add` / `copy`).
    pub fn force(&mut self) -> &mut Self {
        match &mut self.action {
            NotesAction::Add { force, .. } | NotesAction::Copy { force, .. } => *force = true,
            _ => {}
        }
        self
    }

    /// `--allow-empty` — store a note even when the payload is empty (for `add`
    /// / `append`).
    pub fn allow_empty(&mut self) -> &mut Self {
        match &mut self.action {
            NotesAction::Add { allow_empty, .. } | NotesAction::Append { allow_empty, .. } => {
                *allow_empty = true;
            }
            _ => {}
        }
        self
    }

    /// `--no-stripspace` — preserve the payload byte-for-byte instead of
    /// trimming whitespace (for `add` / `append`). Required for exact binary
    /// round-trips.
    pub fn no_stripspace(&mut self) -> &mut Self {
        match &mut self.action {
            NotesAction::Add { no_stripspace, .. } | NotesAction::Append { no_stripspace, .. } => {
                *no_stripspace = true
            }
            _ => {}
        }
        self
    }

    /// `--ignore-missing` — exit zero when there is no note to remove (for
    /// `remove`).
    pub fn ignore_missing(&mut self) -> &mut Self {
        if let NotesAction::Remove { ignore_missing, .. } = &mut self.action {
            *ignore_missing = true;
        }
        self
    }

    /// `-n` dry run (for `prune`).
    pub fn dry_run(&mut self) -> &mut Self {
        if let NotesAction::Prune { dry_run, .. } = &mut self.action {
            *dry_run = true;
        }
        self
    }

    /// `-v` verbose (for `prune`).
    pub fn verbose(&mut self) -> &mut Self {
        if let NotesAction::Prune { verbose, .. } = &mut self.action {
            *verbose = true;
        }
        self
    }
}

#[async_trait]
impl GitCommand for NotesCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["notes".to_string()];
        if let Some(ns) = &self.ref_namespace {
            args.push("--ref".into());
            args.push(ns.clone());
        }
        match &self.action {
            NotesAction::Add {
                object,
                message,
                message_file,
                force,
                allow_empty,
                no_stripspace,
            } => {
                args.push("add".into());
                if *force {
                    args.push("-f".into());
                }
                if *allow_empty {
                    args.push("--allow-empty".into());
                }
                if *no_stripspace {
                    args.push("--no-stripspace".into());
                }
                if let Some(m) = message {
                    args.push("-m".into());
                    args.push(m.clone());
                }
                if let Some(f) = message_file {
                    args.push("-F".into());
                    args.push(f.display().to_string());
                }
                if let Some(o) = object {
                    args.push(o.clone());
                }
            }
            NotesAction::Append {
                object,
                message,
                message_file,
                allow_empty,
                no_stripspace,
            } => {
                args.push("append".into());
                if *allow_empty {
                    args.push("--allow-empty".into());
                }
                if *no_stripspace {
                    args.push("--no-stripspace".into());
                }
                if let Some(m) = message {
                    args.push("-m".into());
                    args.push(m.clone());
                }
                if let Some(f) = message_file {
                    args.push("-F".into());
                    args.push(f.display().to_string());
                }
                if let Some(o) = object {
                    args.push(o.clone());
                }
            }
            NotesAction::Copy { from, to, force } => {
                args.push("copy".into());
                if *force {
                    args.push("-f".into());
                }
                args.push(from.clone());
                args.push(to.clone());
            }
            NotesAction::Show { object } => {
                args.push("show".into());
                if let Some(o) = object {
                    args.push(o.clone());
                }
            }
            NotesAction::List { object } => {
                args.push("list".into());
                if let Some(o) = object {
                    args.push(o.clone());
                }
            }
            NotesAction::Remove {
                object,
                ignore_missing,
            } => {
                args.push("remove".into());
                if *ignore_missing {
                    args.push("--ignore-missing".into());
                }
                if let Some(o) = object {
                    args.push(o.clone());
                }
            }
            NotesAction::Prune { dry_run, verbose } => {
                args.push("prune".into());
                if *dry_run {
                    args.push("-n".into());
                }
                if *verbose {
                    args.push("-v".into());
                }
            }
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        self.execute_raw().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_with_namespace_and_message() {
        let mut c = NotesCommand::add();
        c.ref_namespace("refs/notes/test")
            .object("HEAD")
            .message("hi")
            .force();
        assert_eq!(
            c.build_command_args(),
            vec![
                "notes",
                "--ref",
                "refs/notes/test",
                "add",
                "-f",
                "-m",
                "hi",
                "HEAD"
            ]
        );
    }

    #[test]
    fn add_with_file_and_no_stripspace() {
        let mut c = NotesCommand::add();
        c.object("HEAD")
            .message_file("/tmp/payload.bin")
            .no_stripspace();
        assert_eq!(
            c.build_command_args(),
            vec![
                "notes",
                "add",
                "--no-stripspace",
                "-F",
                "/tmp/payload.bin",
                "HEAD"
            ]
        );
    }

    #[test]
    fn append_payload() {
        let mut c = NotesCommand::append();
        c.object("HEAD").message("more");
        assert_eq!(
            c.build_command_args(),
            vec!["notes", "append", "-m", "more", "HEAD"]
        );
    }

    #[test]
    fn copy_force() {
        let mut c = NotesCommand::copy("a", "b");
        c.force();
        assert_eq!(
            c.build_command_args(),
            vec!["notes", "copy", "-f", "a", "b"]
        );
    }

    #[test]
    fn show_list_remove_prune() {
        let mut s = NotesCommand::show();
        s.ref_namespace("build").object("HEAD");
        assert_eq!(
            s.build_command_args(),
            vec!["notes", "--ref", "build", "show", "HEAD"]
        );

        let mut l = NotesCommand::list();
        l.object("HEAD");
        assert_eq!(l.build_command_args(), vec!["notes", "list", "HEAD"]);

        let mut r = NotesCommand::remove();
        r.object("HEAD").ignore_missing();
        assert_eq!(
            r.build_command_args(),
            vec!["notes", "remove", "--ignore-missing", "HEAD"]
        );

        let mut p = NotesCommand::prune();
        p.dry_run().verbose();
        assert_eq!(p.build_command_args(), vec!["notes", "prune", "-n", "-v"]);
    }
}
