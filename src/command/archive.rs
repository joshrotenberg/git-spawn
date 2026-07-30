//! `git archive` — export a tree to a tar or zip archive.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::path::PathBuf;

/// The archive format `--format=<fmt>` selects.
///
/// The named variants are the formats git ships with. [`Other`](Self::Other)
/// passes a name through verbatim, so a format registered through
/// `tar.<fmt>.command` config can be requested without a variant here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// `tar`: an uncompressed tar archive. git's default when the output file
    /// name carries no recognized extension.
    Tar,
    /// `tar.gz`: a gzip-compressed tar archive.
    TarGz,
    /// `tgz`: the same output as [`TarGz`](Self::TarGz) under git's other name
    /// for it.
    Tgz,
    /// `zip`: a zip archive.
    Zip,
    /// A format name passed through as given.
    Other(String),
}

impl ArchiveFormat {
    /// The name git expects after `--format=`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::Tgz => "tgz",
            Self::Zip => "zip",
            Self::Other(name) => name,
        }
    }
}

/// Builder for `git archive`.
///
/// Writes the contents of a tree-ish to an archive. The tree-ish is required:
/// [`new`](Self::new) takes it, and [`execute`](GitCommand::execute) rejects a
/// command that somehow has none rather than letting git read the option list
/// as one.
///
/// Without [`output`](Self::output) the archive goes to stdout, where it lands
/// in [`CommandOutput::stdout`] as raw bytes. Archives are binary, so read them
/// with [`stdout_bytes`](CommandOutput::stdout_bytes); the lossy
/// [`stdout_str`](CommandOutput::stdout_str) view would corrupt them.
///
/// When neither [`format`](Self::format) nor [`format_raw`](Self::format_raw)
/// is set, git infers the format from the `output` file name's extension and
/// falls back to `tar`.
#[derive(Debug, Clone, Default)]
pub struct ArchiveCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The tree-ish to export.
    pub tree_ish: Option<String>,
    /// `--format=<fmt>`.
    pub format: Option<ArchiveFormat>,
    /// `--prefix=<prefix>`: prepended to every path in the archive.
    pub prefix: Option<String>,
    /// `-o <file>`: write to this file instead of stdout.
    pub output: Option<PathBuf>,
    /// Pathspecs limiting which paths of the tree are included.
    pub paths: Vec<PathBuf>,
}

impl ArchiveCommand {
    /// New command exporting `tree_ish` (a commit, tag or tree).
    pub fn new(tree_ish: impl Into<String>) -> Self {
        Self {
            tree_ish: Some(tree_ish.into()),
            ..Self::default()
        }
    }

    /// Set the archive format (`--format=<fmt>`).
    pub fn format(&mut self, format: ArchiveFormat) -> &mut Self {
        self.format = Some(format);
        self
    }

    /// Set the archive format by name (`--format=<fmt>`), for a format
    /// registered through `tar.<fmt>.command` config.
    pub fn format_raw(&mut self, format: impl Into<String>) -> &mut Self {
        self.format = Some(ArchiveFormat::Other(format.into()));
        self
    }

    /// Prepend `prefix` to every path in the archive (`--prefix=<prefix>`).
    ///
    /// git concatenates the prefix onto each path without inserting a
    /// separator, so include a trailing `/` to nest the contents under a
    /// directory.
    pub fn prefix(&mut self, prefix: impl Into<String>) -> &mut Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Write the archive to `path` instead of stdout (`-o <file>`).
    pub fn output(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.output = Some(path.into());
        self
    }

    /// Limit the archive to paths matching this pathspec. Repeatable.
    pub fn path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.paths.push(path.into());
        self
    }
}

#[async_trait]
impl GitCommand for ArchiveCommand {
    /// Raw output. Without `-o` the archive itself is on `stdout`.
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["archive".to_string()];
        if let Some(format) = &self.format {
            args.push(format!("--format={}", format.as_str()));
        }
        if let Some(prefix) = &self.prefix {
            args.push(format!("--prefix={prefix}"));
        }
        if let Some(output) = &self.output {
            args.push("-o".into());
            args.push(output.display().to_string());
        }
        if let Some(tree_ish) = &self.tree_ish {
            args.push(tree_ish.clone());
        }
        args.extend(self.paths.iter().map(|p| p.display().to_string()));
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        if self.tree_ish.is_none() {
            return Err(Error::invalid_config("archive requires a tree-ish"));
        }
        self.execute_raw().await
    }
}
