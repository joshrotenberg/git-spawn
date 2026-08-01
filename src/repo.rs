//! High-level handle for operating on a git repository.
//!
//! A [`Repository`] is a cheap, cloneable reference to a working tree path.
//! It is the entry point for most users: construct one via
//! [`Repository::open`], [`Repository::init`], or [`Repository::clone`], then
//! call the accessor methods ([`Repository::add`], [`Repository::commit`],
//! [`Repository::log`], ...) to build commands pre-scoped to this repo.
//!
//! ```no_run
//! use git_spawn::{GitCommand, Repository};
//!
//! # async fn example() -> git_spawn::Result<()> {
//! // Create a fresh repo and commit a file into it.
//! let repo = Repository::init("/tmp/demo").await?;
//! std::fs::write(repo.path().join("hello.txt"), "hi")?;
//! repo.add().path("hello.txt").execute().await?;
//! repo.commit().message("first").execute().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Cloning an existing repo
//!
//! ```no_run
//! # use git_spawn::Repository;
//! # async fn example() -> git_spawn::Result<()> {
//! let repo = Repository::clone(
//!     "https://github.com/octocat/Hello-World.git",
//!     "/tmp/hello-world",
//! ).await?;
//! assert!(repo.git_dir().exists());
//! # Ok(())
//! # }
//! ```

use crate::command::{
    GitCommand, add::AddCommand, am::AmCommand, apply::ApplyCommand, archive::ArchiveCommand,
    bisect::BisectCommand, blame::BlameCommand, branch::BranchCommand, bundle::BundleCommand,
    cat_file::CatFileCommand, check_attr::CheckAttrCommand, check_ignore::CheckIgnoreCommand,
    check_ref_format::CheckRefFormatCommand, checkout::CheckoutCommand, cherry::CherryCommand,
    cherry_pick::CherryPickCommand, clean::CleanCommand, clone::CloneCommand,
    commit::CommitCommand, commit_tree::CommitTreeCommand, config::ConfigCommand,
    count_objects::CountObjectsCommand, describe::DescribeCommand, diff::DiffCommand,
    diff_files::DiffFilesCommand, diff_index::DiffIndexCommand, diff_tree::DiffTreeCommand,
    fetch::FetchCommand, for_each_ref::ForEachRefCommand, format_patch::FormatPatchCommand,
    fsck::FsckCommand, gc::GcCommand, grep::GrepCommand, hash_object::HashObjectCommand,
    init::InitCommand, interpret_trailers::InterpretTrailersCommand, log::LogCommand,
    ls_files::LsFilesCommand, ls_remote::LsRemoteCommand, ls_tree::LsTreeCommand,
    maintenance::MaintenanceCommand, merge::MergeCommand, merge_base::MergeBaseCommand,
    merge_file::MergeFileCommand, merge_tree::MergeTreeCommand, mktree::MkTreeCommand,
    mv::MvCommand, name_rev::NameRevCommand, notes::NotesCommand, pull::PullCommand,
    push::PushCommand, range_diff::RangeDiffCommand, read_tree::ReadTreeCommand,
    rebase::RebaseCommand, reflog::ReflogCommand, remote::RemoteCommand, rerere::RerereCommand,
    reset::ResetCommand, restore::RestoreCommand, rev_list::RevListCommand,
    rev_parse::RevParseCommand, revert::RevertCommand, rm::RmCommand, shortlog::ShortlogCommand,
    show::ShowCommand, show_ref::ShowRefCommand, sparse_checkout::SparseCheckoutCommand,
    stash::StashCommand, status::StatusCommand, submodule::SubmoduleCommand, switch::SwitchCommand,
    symbolic_ref::SymbolicRefCommand, tag::TagCommand, update_index::UpdateIndexCommand,
    update_ref::UpdateRefCommand, var::VarCommand, verify_commit::VerifyCommitCommand,
    verify_tag::VerifyTagCommand, worktree::WorktreeCommand, write_tree::WriteTreeCommand,
};
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// A handle to a git working tree.
///
/// Construction does not spawn `git`. [`Repository::open`] only verifies that
/// a `.git` directory (or file, for worktrees/submodules) exists at the path.
#[derive(Debug, Clone)]
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    /// Open an existing repository at `path` without running `git`.
    ///
    /// Returns [`Error::NotARepository`] if `path/.git` does not exist.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let dotgit = path.join(".git");
        if !dotgit.exists() {
            return Err(Error::not_a_repository(path.display().to_string()));
        }
        Ok(Self { path })
    }

    /// Construct a [`Repository`] for `path` without checking that it exists.
    ///
    /// Use this when you are about to run `init` or `clone` into the path.
    #[must_use]
    pub fn new_unchecked(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Working-tree path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Path to the `.git` directory (or file) inside the working tree.
    #[must_use]
    pub fn git_dir(&self) -> PathBuf {
        self.path.join(".git")
    }

    /// Initialize a new repository at `path`.
    ///
    /// Equivalent to `git init <path>`. Returns the created [`Repository`].
    pub async fn init(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent).map_err(Error::from)?;
            }
        }
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(Error::from)?;
        }
        InitCommand::in_directory(path).execute().await
    }

    /// Clone `url` into `path`.
    pub async fn clone(url: impl Into<String>, path: impl Into<PathBuf>) -> Result<Self> {
        let mut cmd = CloneCommand::new(url);
        cmd.directory(path);
        cmd.execute().await
    }

    /// Build an [`AddCommand`] scoped to this repository.
    #[must_use]
    pub fn add(&self) -> AddCommand {
        let mut c = AddCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`CommitCommand`] scoped to this repository.
    #[must_use]
    pub fn commit(&self) -> CommitCommand {
        let mut c = CommitCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`StatusCommand`] scoped to this repository.
    #[must_use]
    pub fn status(&self) -> StatusCommand {
        let mut c = StatusCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`LogCommand`] scoped to this repository.
    #[must_use]
    pub fn log(&self) -> LogCommand {
        let mut c = LogCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`DiffCommand`] scoped to this repository.
    #[must_use]
    pub fn diff(&self) -> DiffCommand {
        let mut c = DiffCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ShowCommand`] scoped to this repository.
    #[must_use]
    pub fn show(&self) -> ShowCommand {
        let mut c = ShowCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`BranchCommand`] scoped to this repository.
    #[must_use]
    pub fn branch(&self) -> BranchCommand {
        let mut c = BranchCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`CheckoutCommand`] scoped to this repository.
    #[must_use]
    pub fn checkout(&self) -> CheckoutCommand {
        let mut c = CheckoutCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`SwitchCommand`] scoped to this repository.
    #[must_use]
    pub fn switch(&self) -> SwitchCommand {
        let mut c = SwitchCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`MergeCommand`] scoped to this repository.
    #[must_use]
    pub fn merge(&self) -> MergeCommand {
        let mut c = MergeCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`RebaseCommand`] scoped to this repository.
    #[must_use]
    pub fn rebase(&self) -> RebaseCommand {
        let mut c = RebaseCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`PullCommand`] scoped to this repository.
    #[must_use]
    pub fn pull(&self) -> PullCommand {
        let mut c = PullCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`PushCommand`] scoped to this repository.
    #[must_use]
    pub fn push(&self) -> PushCommand {
        let mut c = PushCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`FetchCommand`] scoped to this repository.
    #[must_use]
    pub fn fetch(&self) -> FetchCommand {
        let mut c = FetchCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`RemoteCommand`] scoped to this repository.
    #[must_use]
    pub fn remote(&self, action: RemoteCommand) -> RemoteCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`TagCommand`] scoped to this repository.
    #[must_use]
    pub fn tag(&self) -> TagCommand {
        let mut c = TagCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`NotesCommand`] scoped to this repository.
    ///
    /// Construct `action` with [`NotesCommand::add`], [`NotesCommand::append`],
    /// [`NotesCommand::copy`], [`NotesCommand::show`], [`NotesCommand::list`],
    /// [`NotesCommand::remove`], or [`NotesCommand::prune`].
    #[must_use]
    pub fn notes(&self, action: NotesCommand) -> NotesCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`StashCommand`] scoped to this repository.
    #[must_use]
    pub fn stash(&self, action: StashCommand) -> StashCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ResetCommand`] scoped to this repository.
    #[must_use]
    pub fn reset(&self) -> ResetCommand {
        let mut c = ResetCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`RestoreCommand`] scoped to this repository.
    #[must_use]
    pub fn restore(&self) -> RestoreCommand {
        let mut c = RestoreCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`RmCommand`] scoped to this repository.
    #[must_use]
    pub fn rm(&self) -> RmCommand {
        let mut c = RmCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`MvCommand`] scoped to this repository.
    pub fn mv(&self, src: impl Into<String>, dst: impl Into<String>) -> MvCommand {
        let mut c = MvCommand::new(src, dst);
        c.current_dir(&self.path);
        c
    }

    /// Build a [`CherryPickCommand`] scoped to this repository.
    #[must_use]
    pub fn cherry_pick(&self) -> CherryPickCommand {
        let mut c = CherryPickCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`GrepCommand`] scoped to this repository with the given pattern.
    pub fn grep(&self, pattern: impl Into<String>) -> GrepCommand {
        let mut c = GrepCommand::new(pattern);
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ConfigCommand`] scoped to this repository.
    #[must_use]
    pub fn config(&self, action: ConfigCommand) -> ConfigCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ReflogCommand`] scoped to this repository.
    #[must_use]
    pub fn reflog(&self, action: ReflogCommand) -> ReflogCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`WorktreeCommand`] scoped to this repository.
    #[must_use]
    pub fn worktree(&self, action: WorktreeCommand) -> WorktreeCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`SubmoduleCommand`] scoped to this repository.
    #[must_use]
    pub fn submodule(&self, action: SubmoduleCommand) -> SubmoduleCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`BisectCommand`] scoped to this repository.
    #[must_use]
    pub fn bisect(&self, action: BisectCommand) -> BisectCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`RevParseCommand`] scoped to this repository.
    #[must_use]
    pub fn rev_parse(&self) -> RevParseCommand {
        let mut c = RevParseCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`DescribeCommand`] scoped to this repository.
    #[must_use]
    pub fn describe(&self) -> DescribeCommand {
        let mut c = DescribeCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`LsFilesCommand`] scoped to this repository.
    #[must_use]
    pub fn ls_files(&self) -> LsFilesCommand {
        let mut c = LsFilesCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`LsTreeCommand`] for `tree`, scoped to this repository.
    pub fn ls_tree(&self, tree: impl Into<String>) -> LsTreeCommand {
        let mut c = LsTreeCommand::new(tree);
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ShowRefCommand`] scoped to this repository.
    #[must_use]
    pub fn show_ref(&self) -> ShowRefCommand {
        let mut c = ShowRefCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`SymbolicRefCommand`] scoped to this repository.
    ///
    /// Construct `action` with [`SymbolicRefCommand::read`],
    /// [`SymbolicRefCommand::set`], or [`SymbolicRefCommand::delete`].
    #[must_use]
    pub fn symbolic_ref(&self, action: SymbolicRefCommand) -> SymbolicRefCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`CatFileCommand`] scoped to this repository.
    ///
    /// Construct `action` with [`CatFileCommand::pretty_print`],
    /// [`CatFileCommand::object_type`], [`CatFileCommand::size`], or
    /// [`CatFileCommand::exists`]. Use [`CatFileCommand::type_checked`] to
    /// require a particular object type while reading its contents.
    #[must_use]
    pub fn cat_file(&self, action: CatFileCommand) -> CatFileCommand {
        let mut c = action;
        c.current_dir(&self.path);
        c
    }

    /// Build a [`HashObjectCommand`] scoped to this repository.
    #[must_use]
    pub fn hash_object(&self) -> HashObjectCommand {
        let mut c = HashObjectCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build a [`ForEachRefCommand`] scoped to this repository.
    #[must_use]
    pub fn for_each_ref(&self) -> ForEachRefCommand {
        let mut c = ForEachRefCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`UpdateRefCommand`] scoped to this repository.
    #[must_use]
    pub fn update_ref(&self) -> UpdateRefCommand {
        let mut c = UpdateRefCommand::new();
        c.current_dir(&self.path);
        c
    }

    /// Build an [`AmCommand`] scoped to this repository.
    #[must_use]
    pub fn am(&self) -> AmCommand {
        self.scoped(AmCommand::new())
    }

    /// Build an [`ApplyCommand`] scoped to this repository.
    #[must_use]
    pub fn apply(&self) -> ApplyCommand {
        self.scoped(ApplyCommand::new())
    }

    /// Build a [`CheckAttrCommand`] scoped to this repository.
    #[must_use]
    pub fn check_attr(&self) -> CheckAttrCommand {
        self.scoped(CheckAttrCommand::new())
    }

    /// Build a [`CheckIgnoreCommand`] scoped to this repository.
    #[must_use]
    pub fn check_ignore(&self) -> CheckIgnoreCommand {
        self.scoped(CheckIgnoreCommand::new())
    }

    /// Build an [`ArchiveCommand`] for `tree_ish`, scoped to this repository.
    pub fn archive(&self, tree_ish: impl Into<String>) -> ArchiveCommand {
        self.scoped(ArchiveCommand::new(tree_ish))
    }

    /// Build a [`BlameCommand`] scoped to this repository.
    #[must_use]
    pub fn blame(&self) -> BlameCommand {
        self.scoped(BlameCommand::new())
    }

    /// Scope a [`BundleCommand`] action to this repository.
    #[must_use]
    pub fn bundle(&self, action: BundleCommand) -> BundleCommand {
        self.scoped(action)
    }

    /// Build a [`CherryCommand`] scoped to this repository.
    #[must_use]
    pub fn cherry(&self) -> CherryCommand {
        self.scoped(CherryCommand::new())
    }

    /// Build a [`CleanCommand`] scoped to this repository.
    #[must_use]
    pub fn clean(&self) -> CleanCommand {
        self.scoped(CleanCommand::new())
    }

    /// Build a [`CountObjectsCommand`] scoped to this repository.
    #[must_use]
    pub fn count_objects(&self) -> CountObjectsCommand {
        self.scoped(CountObjectsCommand::new())
    }

    /// Build a [`FormatPatchCommand`] scoped to this repository.
    #[must_use]
    pub fn format_patch(&self) -> FormatPatchCommand {
        self.scoped(FormatPatchCommand::new())
    }

    /// Build an [`FsckCommand`] scoped to this repository.
    #[must_use]
    pub fn fsck(&self) -> FsckCommand {
        self.scoped(FsckCommand::new())
    }

    /// Build a [`GcCommand`] scoped to this repository.
    #[must_use]
    pub fn gc(&self) -> GcCommand {
        self.scoped(GcCommand::new())
    }

    /// Build an [`InterpretTrailersCommand`] scoped to this repository.
    #[must_use]
    pub fn interpret_trailers(&self) -> InterpretTrailersCommand {
        self.scoped(InterpretTrailersCommand::new())
    }

    /// Build an [`LsRemoteCommand`] scoped to this repository.
    ///
    /// The default command reads the current branch's configured remote. Use
    /// [`LsRemoteCommand::remote`] directly for a standalone URL or path.
    #[must_use]
    pub fn ls_remote(&self) -> LsRemoteCommand {
        self.scoped(LsRemoteCommand::new())
    }

    /// Scope a [`CheckRefFormatCommand`] to this repository.
    ///
    /// Full ref-name validation is standalone, but branch mode can expand
    /// reflog syntax such as `@{-1}` against the scoped repository.
    #[must_use]
    pub fn check_ref_format(&self, command: CheckRefFormatCommand) -> CheckRefFormatCommand {
        self.scoped(command)
    }

    /// Scope a [`MaintenanceCommand`] action to this repository.
    #[must_use]
    pub fn maintenance(&self, action: MaintenanceCommand) -> MaintenanceCommand {
        self.scoped(action)
    }

    /// Build a [`MergeBaseCommand`] scoped to this repository.
    #[must_use]
    pub fn merge_base(&self) -> MergeBaseCommand {
        self.scoped(MergeBaseCommand::new())
    }

    /// Build a [`NameRevCommand`] scoped to this repository.
    #[must_use]
    pub fn name_rev(&self) -> NameRevCommand {
        self.scoped(NameRevCommand::new())
    }

    /// Build a [`RangeDiffCommand`] scoped to this repository.
    #[must_use]
    pub fn range_diff(&self) -> RangeDiffCommand {
        self.scoped(RangeDiffCommand::new())
    }

    /// Scope a [`RerereCommand`] action to this repository.
    #[must_use]
    pub fn rerere(&self, action: RerereCommand) -> RerereCommand {
        self.scoped(action)
    }

    /// Build a [`RevertCommand`] scoped to this repository.
    #[must_use]
    pub fn revert(&self) -> RevertCommand {
        self.scoped(RevertCommand::new())
    }

    /// Build a [`ShortlogCommand`] scoped to this repository.
    #[must_use]
    pub fn shortlog(&self) -> ShortlogCommand {
        self.scoped(ShortlogCommand::new())
    }

    /// Scope a [`SparseCheckoutCommand`] action to this repository.
    #[must_use]
    pub fn sparse_checkout(&self, action: SparseCheckoutCommand) -> SparseCheckoutCommand {
        self.scoped(action)
    }

    /// Scope a [`VarCommand`] action to this repository.
    #[must_use]
    pub fn var(&self, action: VarCommand) -> VarCommand {
        self.scoped(action)
    }

    /// Build a [`VerifyCommitCommand`] scoped to this repository.
    #[must_use]
    pub fn verify_commit(&self) -> VerifyCommitCommand {
        self.scoped(VerifyCommitCommand::new())
    }

    /// Build a [`VerifyTagCommand`] scoped to this repository.
    #[must_use]
    pub fn verify_tag(&self) -> VerifyTagCommand {
        self.scoped(VerifyTagCommand::new())
    }

    /// Build a [`RevListCommand`] scoped to this repository.
    #[must_use]
    pub fn rev_list(&self) -> RevListCommand {
        self.scoped(RevListCommand::new())
    }
    /// Build a [`CommitTreeCommand`] scoped to this repository.
    #[must_use]
    pub fn commit_tree(&self) -> CommitTreeCommand {
        self.scoped(CommitTreeCommand::new())
    }
    /// Build a [`WriteTreeCommand`] scoped to this repository.
    #[must_use]
    pub fn write_tree(&self) -> WriteTreeCommand {
        self.scoped(WriteTreeCommand::new())
    }
    /// Build a [`ReadTreeCommand`] scoped to this repository.
    #[must_use]
    pub fn read_tree(&self) -> ReadTreeCommand {
        self.scoped(ReadTreeCommand::new())
    }
    /// Build an [`UpdateIndexCommand`] scoped to this repository.
    #[must_use]
    pub fn update_index(&self) -> UpdateIndexCommand {
        self.scoped(UpdateIndexCommand::new())
    }
    /// Build an [`MkTreeCommand`] scoped to this repository.
    #[must_use]
    pub fn mktree(&self) -> MkTreeCommand {
        self.scoped(MkTreeCommand::new())
    }
    /// Build a [`MergeTreeCommand`] scoped to this repository.
    #[must_use]
    pub fn merge_tree(&self) -> MergeTreeCommand {
        self.scoped(MergeTreeCommand::new())
    }
    /// Build a [`MergeFileCommand`] scoped to this repository.
    #[must_use]
    pub fn merge_file(&self) -> MergeFileCommand {
        self.scoped(MergeFileCommand::new())
    }
    /// Build a [`DiffTreeCommand`] scoped to this repository.
    #[must_use]
    pub fn diff_tree(&self) -> DiffTreeCommand {
        self.scoped(DiffTreeCommand::new())
    }
    /// Build a [`DiffIndexCommand`] scoped to this repository.
    #[must_use]
    pub fn diff_index(&self) -> DiffIndexCommand {
        self.scoped(DiffIndexCommand::new())
    }
    /// Build a [`DiffFilesCommand`] scoped to this repository.
    #[must_use]
    pub fn diff_files(&self) -> DiffFilesCommand {
        self.scoped(DiffFilesCommand::new())
    }

    fn scoped<C: GitCommand>(&self, mut command: C) -> C {
        command.current_dir(&self.path);
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_missing_repo_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = Repository::open(tmp.path()).unwrap_err();
        assert!(matches!(err, Error::NotARepository { .. }));
    }

    #[test]
    fn new_unchecked_does_not_check() {
        let repo = Repository::new_unchecked("/definitely/not/here");
        assert_eq!(repo.path(), Path::new("/definitely/not/here"));
    }

    #[test]
    fn object_and_ref_accessors_scope_current_dir() {
        let repo = Repository::new_unchecked("/tmp/some-repo");
        let want = Some(PathBuf::from("/tmp/some-repo"));

        let cat = repo.cat_file(CatFileCommand::pretty_print("HEAD"));
        assert_eq!(cat.get_executor().cwd, want);
        assert_eq!(cat.build_command_args(), vec!["cat-file", "-p", "HEAD"]);

        let hash = repo.hash_object();
        assert_eq!(hash.get_executor().cwd, want);

        let each = repo.for_each_ref();
        assert_eq!(each.get_executor().cwd, want);

        let update = repo.update_ref();
        assert_eq!(update.get_executor().cwd, want);
    }

    #[test]
    fn added_repository_accessors_scope_current_dir_and_build_expected_argv() {
        let repo = Repository::new_unchecked("/tmp/some-repo");
        let want = Some(PathBuf::from("/tmp/some-repo"));

        macro_rules! assert_scoped {
            ($command:expr, [$($arg:expr),* $(,)?]) => {{
                let command = $command;
                assert_eq!(command.get_executor().cwd, want);
                assert_eq!(command.build_command_args(), vec![$($arg),*]);
            }};
        }

        assert_scoped!(repo.am(), ["am"]);
        assert_scoped!(repo.apply(), ["apply"]);
        assert_scoped!(repo.check_attr(), ["check-attr"]);
        assert_scoped!(repo.check_ignore(), ["check-ignore"]);
        assert_scoped!(repo.archive("HEAD"), ["archive", "HEAD"]);
        assert_scoped!(repo.blame(), ["blame"]);
        assert_scoped!(
            repo.bundle(BundleCommand::create("repo.bundle")),
            ["bundle", "create", "repo.bundle"]
        );
        assert_scoped!(repo.cherry(), ["cherry"]);
        assert_scoped!(repo.clean(), ["clean"]);
        assert_scoped!(repo.count_objects(), ["count-objects"]);
        assert_scoped!(repo.format_patch(), ["format-patch"]);
        assert_scoped!(repo.fsck(), ["fsck"]);
        assert_scoped!(repo.gc(), ["gc"]);
        assert_scoped!(repo.interpret_trailers(), ["interpret-trailers"]);
        assert_scoped!(repo.ls_remote(), ["ls-remote"]);
        assert_scoped!(
            repo.check_ref_format(CheckRefFormatCommand::branch("@{-1}")),
            ["check-ref-format", "--branch", "@{-1}"]
        );
        assert_scoped!(
            repo.maintenance(MaintenanceCommand::run()),
            ["maintenance", "run"]
        );
        assert_scoped!(repo.merge_base(), ["merge-base"]);
        assert_scoped!(repo.name_rev(), ["name-rev"]);
        assert_scoped!(repo.range_diff(), ["range-diff"]);
        assert_scoped!(repo.rerere(RerereCommand::status()), ["rerere", "status"]);
        assert_scoped!(repo.revert(), ["revert"]);
        assert_scoped!(repo.shortlog(), ["shortlog"]);
        assert_scoped!(
            repo.sparse_checkout(SparseCheckoutCommand::list()),
            ["sparse-checkout", "list"]
        );
        assert_scoped!(
            repo.var(VarCommand::get("GIT_EDITOR")),
            ["var", "GIT_EDITOR"]
        );
        assert_scoped!(repo.verify_commit(), ["verify-commit"]);
        assert_scoped!(repo.verify_tag(), ["verify-tag"]);
    }
}
