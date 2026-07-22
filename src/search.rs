//! Typed search over `git grep`.
//!
//! Reached through [`Repository::search`], which returns a [`SearchOps`]
//! builder. Configure the pattern and any filters, then call
//! [`execute`](SearchOps::execute) to get one [`Hit`] per matching line.
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! let hits = repo.search().pattern("TODO").in_paths(["src/"]).execute().await?;
//! for hit in hits {
//!     println!("{}:{}: {}", hit.path, hit.line, hit.text);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! The search always runs `git grep -n` against the working tree (or the
//! index with [`cached`](SearchOps::cached)), so every [`Hit`] carries a line
//! number. "No matches" is not an error: it yields an empty `Vec`, matching
//! [`GrepCommand::execute_allow_no_match`](crate::command::grep::GrepCommand::execute_allow_no_match).

use crate::error::{Error, Result};
use crate::repo::Repository;

/// One matching line from a search.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hit {
    /// Path of the file the match was found in, relative to the repository.
    pub path: String,
    /// 1-based line number of the match.
    pub line: u64,
    /// Full text of the matching line, without the trailing newline.
    pub text: String,
}

/// Chained builder over `git grep`, scoped to a [`Repository`].
///
/// Obtained via [`Repository::search`]. Set a [`pattern`](SearchOps::pattern)
/// and any filters, then call [`execute`](SearchOps::execute). The handle
/// borrows the repository for the duration of one chained call.
#[derive(Debug)]
pub struct SearchOps<'a> {
    repo: &'a Repository,
    pattern: Option<String>,
    paths: Vec<String>,
    ignore_case: bool,
    word_regexp: bool,
    fixed_strings: bool,
    extended_regexp: bool,
    perl_regexp: bool,
    cached: bool,
}

impl<'a> SearchOps<'a> {
    fn new(repo: &'a Repository) -> Self {
        Self {
            repo,
            pattern: None,
            paths: Vec::new(),
            ignore_case: false,
            word_regexp: false,
            fixed_strings: false,
            extended_regexp: false,
            perl_regexp: false,
            cached: false,
        }
    }

    /// Set the pattern to search for. Required before [`execute`](SearchOps::execute).
    #[must_use]
    pub fn pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Restrict the search to one path (pathspec). May be called repeatedly.
    #[must_use]
    pub fn in_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }

    /// Restrict the search to several paths (pathspecs).
    #[must_use]
    pub fn in_paths(mut self, paths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.paths.extend(paths.into_iter().map(Into::into));
        self
    }

    /// Match case-insensitively (`-i`).
    #[must_use]
    pub fn case_insensitive(mut self) -> Self {
        self.ignore_case = true;
        self
    }

    /// Match the pattern only at word boundaries (`-w`).
    #[must_use]
    pub fn word_regexp(mut self) -> Self {
        self.word_regexp = true;
        self
    }

    /// Treat the pattern as a fixed string rather than a regex (`-F`).
    #[must_use]
    pub fn fixed_strings(mut self) -> Self {
        self.fixed_strings = true;
        self
    }

    /// Interpret the pattern as an extended POSIX regex (`-E`).
    #[must_use]
    pub fn extended_regexp(mut self) -> Self {
        self.extended_regexp = true;
        self
    }

    /// Interpret the pattern as a Perl-compatible regex (`-P`).
    #[must_use]
    pub fn perl_regexp(mut self) -> Self {
        self.perl_regexp = true;
        self
    }

    /// Search the index instead of the working tree (`--cached`).
    #[must_use]
    pub fn cached(mut self) -> Self {
        self.cached = true;
        self
    }

    /// Run the search and return one [`Hit`] per matching line.
    ///
    /// # Errors
    /// Returns [`Error::InvalidConfig`] if no [`pattern`](SearchOps::pattern)
    /// was set, or an error if the underlying `git grep` fails for a reason
    /// other than "no matches" (for example a bad pathspec), or if its output
    /// cannot be parsed.
    pub async fn execute(self) -> Result<Vec<Hit>> {
        let pattern = self
            .pattern
            .ok_or_else(|| Error::invalid_config("search requires a pattern"))?;

        let mut cmd = self.repo.grep(pattern);
        cmd.line_number();
        if self.ignore_case {
            cmd.ignore_case();
        }
        if self.word_regexp {
            cmd.word_regexp();
        }
        if self.fixed_strings {
            cmd.fixed_strings();
        }
        if self.extended_regexp {
            cmd.extended_regexp();
        }
        if self.perl_regexp {
            cmd.perl_regexp();
        }
        if self.cached {
            cmd.cached();
        }
        for p in &self.paths {
            cmd.path(p);
        }

        match cmd.execute_allow_no_match().await? {
            None => Ok(Vec::new()),
            Some(out) => parse_hits(&out.stdout_str()),
        }
    }
}

impl Repository {
    /// Typed search over `git grep`.
    #[must_use]
    pub fn search(&self) -> SearchOps<'_> {
        SearchOps::new(self)
    }
}

/// Parse `git grep -n` output (`<path>:<line>:<text>`) into [`Hit`]s.
///
/// Each non-empty line carries a path, a line number, and the matching text.
/// The path and line number are split off on the first two colons; everything
/// after the second colon is the matching text, so colons in the text are
/// preserved.
fn parse_hits(stdout: &str) -> Result<Vec<Hit>> {
    let mut hits = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, ':');
        let path = parts
            .next()
            .ok_or_else(|| Error::parse_error(format!("grep line has no path: {line:?}")))?;
        let line_no = parts
            .next()
            .ok_or_else(|| Error::parse_error(format!("grep line has no line number: {line:?}")))?;
        let text = parts
            .next()
            .ok_or_else(|| Error::parse_error(format!("grep line has no text: {line:?}")))?;
        let line_no: u64 = line_no.parse().map_err(|_| {
            Error::parse_error(format!("grep line number is not a number: {line:?}"))
        })?;
        hits.push(Hit {
            path: path.to_string(),
            line: line_no,
            text: text.to_string(),
        });
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_hit() {
        let input = "src/lib.rs:42:    // TODO: fix this\n";
        let hits = parse_hits(input).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "src/lib.rs");
        assert_eq!(hits[0].line, 42);
        assert_eq!(hits[0].text, "    // TODO: fix this");
    }

    #[test]
    fn preserves_colons_in_text() {
        let input = "src/main.rs:7:let url = \"https://example.com\";\n";
        let hits = parse_hits(input).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "src/main.rs");
        assert_eq!(hits[0].line, 7);
        assert_eq!(hits[0].text, "let url = \"https://example.com\";");
    }

    #[test]
    fn parses_multiple_hits_in_order() {
        let input = "a.rs:1:one\nb.rs:2:two\nb.rs:5:five\n";
        let hits = parse_hits(input).unwrap();
        let locs: Vec<(&str, u64)> = hits.iter().map(|h| (h.path.as_str(), h.line)).collect();
        assert_eq!(locs, vec![("a.rs", 1), ("b.rs", 2), ("b.rs", 5)]);
    }

    #[test]
    fn empty_output_is_empty() {
        assert!(parse_hits("").unwrap().is_empty());
    }

    #[test]
    fn non_numeric_line_errors() {
        assert!(parse_hits("a.rs:xx:text\n").is_err());
    }

    #[test]
    fn line_without_line_number_errors() {
        assert!(parse_hits("a.rs\n").is_err());
    }
}
