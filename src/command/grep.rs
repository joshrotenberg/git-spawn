//! `git grep` — print lines matching a pattern.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git grep`.
#[derive(Debug, Clone, Default)]
pub struct GrepCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Pattern to search for.
    pub pattern: Option<String>,
    /// Tree-ishes to search (defaults to working tree if empty).
    pub trees: Vec<String>,
    /// Pathspecs.
    pub paths: Vec<String>,
    /// `-i` case-insensitive.
    pub ignore_case: bool,
    /// `-w` match whole word.
    pub word_regexp: bool,
    /// `-v` invert match.
    pub invert: bool,
    /// `-n` show line numbers.
    pub line_number: bool,
    /// `-c` count matches per file.
    pub count: bool,
    /// `-l` / `--files-with-matches`.
    pub files_with_matches: bool,
    /// `-L` / `--files-without-match`.
    pub files_without_match: bool,
    /// `--name-only`.
    pub name_only: bool,
    /// `-E` extended regex.
    pub extended_regexp: bool,
    /// `-F` fixed string.
    pub fixed_strings: bool,
    /// `-P` Perl regex.
    pub perl_regexp: bool,
    /// `--cached`.
    pub cached: bool,
    /// `--untracked`.
    pub untracked: bool,
    /// `--no-index`.
    pub no_index: bool,
    /// `--recurse-submodules`.
    pub recurse_submodules: bool,
}

impl GrepCommand {
    /// New grep with the given pattern.
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: Some(pattern.into()),
            ..Self::default()
        }
    }

    /// Search a tree-ish (e.g. `HEAD`, a commit, a branch).
    pub fn tree(&mut self, t: impl Into<String>) -> &mut Self {
        self.trees.push(t.into());
        self
    }

    /// Filter by path.
    pub fn path(&mut self, p: impl Into<String>) -> &mut Self {
        self.paths.push(p.into());
        self
    }

    /// `-i`.
    pub fn ignore_case(&mut self) -> &mut Self {
        self.ignore_case = true;
        self
    }

    /// `-w`.
    pub fn word_regexp(&mut self) -> &mut Self {
        self.word_regexp = true;
        self
    }

    /// `-v`.
    pub fn invert(&mut self) -> &mut Self {
        self.invert = true;
        self
    }

    /// `-n`.
    pub fn line_number(&mut self) -> &mut Self {
        self.line_number = true;
        self
    }

    /// `-c`.
    pub fn count(&mut self) -> &mut Self {
        self.count = true;
        self
    }

    /// `-l`.
    pub fn files_with_matches(&mut self) -> &mut Self {
        self.files_with_matches = true;
        self
    }

    /// `-L`.
    pub fn files_without_match(&mut self) -> &mut Self {
        self.files_without_match = true;
        self
    }

    /// `--name-only`.
    pub fn name_only(&mut self) -> &mut Self {
        self.name_only = true;
        self
    }

    /// `-E`.
    pub fn extended_regexp(&mut self) -> &mut Self {
        self.extended_regexp = true;
        self
    }

    /// `-F`.
    pub fn fixed_strings(&mut self) -> &mut Self {
        self.fixed_strings = true;
        self
    }

    /// `-P`.
    pub fn perl_regexp(&mut self) -> &mut Self {
        self.perl_regexp = true;
        self
    }

    /// `--cached`.
    pub fn cached(&mut self) -> &mut Self {
        self.cached = true;
        self
    }

    /// `--untracked`.
    pub fn untracked(&mut self) -> &mut Self {
        self.untracked = true;
        self
    }

    /// `--no-index`.
    pub fn no_index(&mut self) -> &mut Self {
        self.no_index = true;
        self
    }

    /// `--recurse-submodules`.
    pub fn recurse_submodules(&mut self) -> &mut Self {
        self.recurse_submodules = true;
        self
    }
}

#[async_trait]
impl GitCommand for GrepCommand {
    type Output = CommandOutput;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["grep".to_string()];
        if self.ignore_case {
            args.push("-i".into());
        }
        if self.word_regexp {
            args.push("-w".into());
        }
        if self.invert {
            args.push("-v".into());
        }
        if self.line_number {
            args.push("-n".into());
        }
        if self.count {
            args.push("-c".into());
        }
        if self.files_with_matches {
            args.push("-l".into());
        }
        if self.files_without_match {
            args.push("-L".into());
        }
        if self.name_only {
            args.push("--name-only".into());
        }
        if self.extended_regexp {
            args.push("-E".into());
        }
        if self.fixed_strings {
            args.push("-F".into());
        }
        if self.perl_regexp {
            args.push("-P".into());
        }
        if self.cached {
            args.push("--cached".into());
        }
        if self.untracked {
            args.push("--untracked".into());
        }
        if self.no_index {
            args.push("--no-index".into());
        }
        if self.recurse_submodules {
            args.push("--recurse-submodules".into());
        }
        if let Some(p) = &self.pattern {
            args.push("-e".into());
            args.push(p.clone());
        }
        args.extend(self.trees.iter().cloned());
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }
    async fn execute(&self) -> Result<CommandOutput> {
        if self.pattern.is_none() {
            return Err(Error::invalid_config("grep requires a pattern"));
        }
        // `git grep` exits 1 on "no matches" which we surface as CommandFailed.
        // Callers that want to distinguish should match on Error::CommandFailed.
        self.execute_raw().await
    }
}
