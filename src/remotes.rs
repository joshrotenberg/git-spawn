//! Typed listing and management of remotes.
//!
//! Reached through [`Repository::remotes`], which returns a [`RemoteOps`]
//! handle:
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Every configured remote with its fetch and push URLs.
//! for remote in repo.remotes().list().await? {
//!     println!("{}  {}  (push: {})", remote.name, remote.fetch_url, remote.push_url);
//! }
//!
//! repo.remotes().add("upstream", "https://example.com/up.git").await?;
//! repo.remotes().set_url("upstream", "git@example.com:up.git").await?;
//! let url = repo.remotes().get_url("upstream").await?;
//! repo.remotes().rename("upstream", "up").await?;
//! repo.remotes().remove("up").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Listing parses `git remote -v`, whose output pairs a fetch and a push line
//! per remote. The parser groups the two lines by remote name, so a [`Remote`]
//! always carries both URLs.

use crate::command::GitCommand;
use crate::command::remote::RemoteCommand;
use crate::error::Result;
use crate::repo::Repository;

/// One configured remote with its fetch and push URLs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Remote {
    /// Remote name (e.g. `"origin"`).
    pub name: String,
    /// URL used for fetching.
    pub fetch_url: String,
    /// URL used for pushing. Equals [`fetch_url`](Remote::fetch_url) unless a
    /// separate push URL is configured.
    pub push_url: String,
}

/// Operations on remotes, scoped to a [`Repository`].
///
/// Obtained via [`Repository::remotes`]. The handle borrows the repository
/// for the duration of one chained call — there is no shared state.
#[derive(Debug)]
pub struct RemoteOps<'a> {
    repo: &'a Repository,
}

impl<'a> RemoteOps<'a> {
    /// List every configured remote with its fetch and push URLs.
    ///
    /// # Errors
    /// Returns an error if the `git remote -v` invocation fails or its output
    /// cannot be parsed.
    pub async fn list(&self) -> Result<Vec<Remote>> {
        let out = self
            .repo
            .remote(RemoteCommand::list_verbose())
            .execute()
            .await?;
        parse_remotes(&out.stdout_str())
    }

    /// Add a remote `name` pointing at `url` (`git remote add`).
    ///
    /// # Errors
    /// Returns an error if the `git remote add` invocation fails (for example
    /// when a remote of that name already exists).
    pub async fn add(&self, name: impl Into<String>, url: impl Into<String>) -> Result<()> {
        self.repo
            .remote(RemoteCommand::add(name, url))
            .execute()
            .await?;
        Ok(())
    }

    /// Remove the remote `name` (`git remote remove`).
    ///
    /// # Errors
    /// Returns an error if the `git remote remove` invocation fails (for
    /// example when no such remote exists).
    pub async fn remove(&self, name: impl Into<String>) -> Result<()> {
        self.repo
            .remote(RemoteCommand::remove(name))
            .execute()
            .await?;
        Ok(())
    }

    /// Rename the remote `from` to `to` (`git remote rename`).
    ///
    /// # Errors
    /// Returns an error if the `git remote rename` invocation fails.
    pub async fn rename(&self, from: impl Into<String>, to: impl Into<String>) -> Result<()> {
        self.repo
            .remote(RemoteCommand::rename(from, to))
            .execute()
            .await?;
        Ok(())
    }

    /// Change the URL of remote `name` (`git remote set-url`).
    ///
    /// # Errors
    /// Returns an error if the `git remote set-url` invocation fails.
    pub async fn set_url(&self, name: impl Into<String>, url: impl Into<String>) -> Result<()> {
        self.repo
            .remote(RemoteCommand::set_url(name, url))
            .execute()
            .await?;
        Ok(())
    }

    /// Fetch URL of the remote `name`.
    ///
    /// This reads the fetch URL from [`list`](RemoteOps::list), which matches
    /// the default of `git remote get-url`.
    ///
    /// # Errors
    /// Returns an error if the underlying listing fails or no remote named
    /// `name` is configured.
    pub async fn get_url(&self, name: impl AsRef<str>) -> Result<String> {
        let name = name.as_ref();
        self.list()
            .await?
            .into_iter()
            .find(|r| r.name == name)
            .map(|r| r.fetch_url)
            .ok_or_else(|| crate::error::Error::parse_error(format!("no remote named {name:?}")))
    }
}

impl Repository {
    /// Operations on remotes.
    #[must_use]
    pub fn remotes(&self) -> RemoteOps<'_> {
        RemoteOps { repo: self }
    }
}

/// Parse the output of `git remote -v` into one [`Remote`] per name.
///
/// Each remote contributes two lines, `<name>\t<url> (fetch)` and
/// `<name>\t<url> (push)`; they are grouped by name so both URLs land on the
/// same record. Records keep first-seen order.
fn parse_remotes(stdout: &str) -> Result<Vec<Remote>> {
    let mut out: Vec<Remote> = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let (name, rest) = line.split_once('\t').ok_or_else(|| {
            crate::error::Error::parse_error(format!("remote line has no tab: {line:?}"))
        })?;
        let (url, kind) = rest.rsplit_once(' ').ok_or_else(|| {
            crate::error::Error::parse_error(format!("remote line has no direction: {line:?}"))
        })?;

        let entry = match out.iter_mut().find(|r| r.name == name) {
            Some(existing) => existing,
            None => {
                out.push(Remote {
                    name: name.to_string(),
                    ..Remote::default()
                });
                out.last_mut().expect("just pushed")
            }
        };
        match kind {
            "(fetch)" => entry.fetch_url = url.to_string(),
            "(push)" => entry.push_url = url.to_string(),
            other => {
                return Err(crate::error::Error::parse_error(format!(
                    "unexpected remote direction {other:?}: {line:?}"
                )));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_remote() {
        let input = "origin\thttps://example.com/x.git (fetch)\n\
                     origin\thttps://example.com/x.git (push)\n";
        let remotes = parse_remotes(input).unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(remotes[0].fetch_url, "https://example.com/x.git");
        assert_eq!(remotes[0].push_url, "https://example.com/x.git");
    }

    #[test]
    fn parses_distinct_push_url() {
        let input = "origin\thttps://example.com/x.git (fetch)\n\
                     origin\tgit@example.com:x.git (push)\n";
        let remotes = parse_remotes(input).unwrap();
        assert_eq!(remotes.len(), 1);
        assert_eq!(remotes[0].fetch_url, "https://example.com/x.git");
        assert_eq!(remotes[0].push_url, "git@example.com:x.git");
    }

    #[test]
    fn parses_multiple_remotes_in_order() {
        let input = "origin\thttps://example.com/o.git (fetch)\n\
                     origin\thttps://example.com/o.git (push)\n\
                     upstream\thttps://example.com/u.git (fetch)\n\
                     upstream\thttps://example.com/u.git (push)\n";
        let remotes = parse_remotes(input).unwrap();
        let names: Vec<&str> = remotes.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["origin", "upstream"]);
        assert_eq!(remotes[1].fetch_url, "https://example.com/u.git");
    }

    #[test]
    fn empty_output_is_empty() {
        assert!(parse_remotes("").unwrap().is_empty());
    }

    #[test]
    fn line_without_tab_errors() {
        assert!(parse_remotes("origin https://example.com/x.git (fetch)\n").is_err());
    }
}
