//! Typed listing and bulk operations on tags.
//!
//! Reached through [`Repository::tags`], which returns a [`TagOps`] handle:
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//! use git_spawn::tags::TagKind;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! for tag in repo.tags().list().await? {
//!     match tag.kind {
//!         TagKind::Annotated => println!("{} (annotated): {}",
//!             tag.name,
//!             tag.message.as_deref().unwrap_or("")),
//!         TagKind::Lightweight => println!("{} (lightweight) -> {}", tag.name, tag.target),
//!     }
//! }
//!
//! repo.tags().create("v0.1.0", "HEAD").await?;
//! repo.tags().create_annotated("v0.2.0", "HEAD", "release 0.2.0").await?;
//! # Ok(())
//! # }
//! ```
//!
//! Like [`crate::branches`], listing uses `for-each-ref` with a fixed
//! NUL-delimited format string.

use crate::command::GitCommand;
use crate::command::for_each_ref::ForEachRefCommand;
use crate::command::tag::TagCommand;
use crate::error::Result;
use crate::repo::Repository;

/// One tag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tag {
    /// Short tag name (e.g. `"v1.2.3"`).
    pub name: String,
    /// Whether the tag is a plain ref (lightweight) or carries its own object
    /// (annotated).
    pub kind: TagKind,
    /// Short SHA of the commit the tag ultimately points at.
    pub target: String,
    /// Subject line of the tag message — `None` for lightweight tags.
    pub message: Option<String>,
    /// Tagger metadata — `None` for lightweight tags.
    pub tagger: Option<Tagger>,
}

/// Lightweight vs. annotated tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TagKind {
    /// Plain ref pointing at a commit.
    Lightweight,
    /// Tag object with its own metadata (message, tagger, date).
    Annotated,
}

/// Tagger identity attached to an annotated tag.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tagger {
    /// Tagger display name.
    pub name: String,
    /// Tagger email (with `<>` stripped).
    pub email: String,
    /// Tag date, RFC 3339 / ISO 8601 strict.
    pub date: String,
}

/// Operations on tags, scoped to a [`Repository`].
#[derive(Debug)]
pub struct TagOps<'a> {
    repo: &'a Repository,
}

impl<'a> TagOps<'a> {
    /// List every tag in the repository.
    pub async fn list(&self) -> Result<Vec<Tag>> {
        self.list_inner(None).await
    }

    /// List tags whose ref path matches `pattern`
    /// (e.g. `"refs/tags/v1.*"`).
    pub async fn list_matching(&self, pattern: impl Into<String>) -> Result<Vec<Tag>> {
        self.list_inner(Some(pattern.into())).await
    }

    /// Create a lightweight tag `name` pointing at `target` (a commit-ish).
    pub async fn create(&self, name: impl Into<String>, target: impl Into<String>) -> Result<()> {
        let mut cmd = TagCommand::new();
        cmd.name(name).commit(target);
        cmd.current_dir(self.repo.path());
        cmd.execute().await?;
        Ok(())
    }

    /// Create an annotated tag `name` pointing at `target` with `message`.
    pub async fn create_annotated(
        &self,
        name: impl Into<String>,
        target: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<()> {
        let mut cmd = TagCommand::new();
        cmd.name(name).commit(target).message(message);
        cmd.current_dir(self.repo.path());
        cmd.execute().await?;
        Ok(())
    }

    /// Delete the tag `name`.
    pub async fn delete(&self, name: impl Into<String>) -> Result<()> {
        let mut cmd = TagCommand::new();
        cmd.name(name).delete();
        cmd.current_dir(self.repo.path());
        cmd.execute().await?;
        Ok(())
    }

    async fn list_inner(&self, pattern: Option<String>) -> Result<Vec<Tag>> {
        let mut cmd = ForEachRefCommand::new();
        cmd.format(FORMAT.to_string())
            .pattern(pattern.unwrap_or_else(|| "refs/tags/".to_string()));
        cmd.current_dir(self.repo.path());
        let out = cmd.execute().await?;
        parse_tags(&out.stdout_str())
    }
}

impl Repository {
    /// Operations on tags.
    #[must_use]
    pub fn tags(&self) -> TagOps<'_> {
        TagOps { repo: self }
    }
}

/// NUL-delimited per-record format. Field order matches [`parse_tags`].
///
/// Fields: name, objecttype, objectname:short, *objectname:short,
/// contents:subject, taggername, taggeremail, taggerdate.
const FORMAT: &str = concat!(
    "%(refname:short)",
    "%00",
    "%(objecttype)",
    "%00",
    "%(objectname:short)",
    "%00",
    "%(*objectname:short)",
    "%00",
    "%(contents:subject)",
    "%00",
    "%(taggername)",
    "%00",
    "%(taggeremail)",
    "%00",
    "%(taggerdate:iso-strict)",
);

fn parse_tags(stdout: &str) -> Result<Vec<Tag>> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\0').collect();
        if fields.len() < 8 {
            return Err(crate::error::Error::parse_error(format!(
                "tag record has {} fields, expected 8: {line:?}",
                fields.len()
            )));
        }
        let kind = if fields[1] == "tag" {
            TagKind::Annotated
        } else {
            TagKind::Lightweight
        };
        let target = match kind {
            TagKind::Annotated => fields[3].to_string(),
            TagKind::Lightweight => fields[2].to_string(),
        };
        let (message, tagger) = match kind {
            TagKind::Annotated => {
                let msg = if fields[4].is_empty() {
                    None
                } else {
                    Some(fields[4].to_string())
                };
                let email = fields[6]
                    .trim_start_matches('<')
                    .trim_end_matches('>')
                    .to_string();
                let tagger = if fields[5].is_empty() && email.is_empty() {
                    None
                } else {
                    Some(Tagger {
                        name: fields[5].to_string(),
                        email,
                        date: fields[7].to_string(),
                    })
                };
                (msg, tagger)
            }
            TagKind::Lightweight => (None, None),
        };
        out.push(Tag {
            name: fields[0].to_string(),
            kind,
            target,
            message,
            tagger,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lightweight_tag() {
        let input = "v0.1\0commit\0abc1234\0\0\0\0\0\n";
        let tags = parse_tags(input).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v0.1");
        assert_eq!(tags[0].kind, TagKind::Lightweight);
        assert_eq!(tags[0].target, "abc1234");
        assert!(tags[0].message.is_none());
        assert!(tags[0].tagger.is_none());
    }

    #[test]
    fn parses_annotated_tag() {
        let input = "v1.0\0tag\0deadbeef\0abc1234\0release 1.0\0Alice\0<alice@example.com>\x002026-04-01T12:00:00+00:00\n";
        let tags = parse_tags(input).unwrap();
        assert_eq!(tags.len(), 1);
        let t = &tags[0];
        assert_eq!(t.name, "v1.0");
        assert_eq!(t.kind, TagKind::Annotated);
        assert_eq!(t.target, "abc1234");
        assert_eq!(t.message.as_deref(), Some("release 1.0"));
        let tagger = t.tagger.as_ref().unwrap();
        assert_eq!(tagger.name, "Alice");
        assert_eq!(tagger.email, "alice@example.com");
        assert_eq!(tagger.date, "2026-04-01T12:00:00+00:00");
    }

    #[test]
    fn parses_mixed_records() {
        let input = concat!(
            "lw\0commit\0aaaaaaa\0\0\0\0\0\n",
            "ann\0tag\0bbbbbbb\0ccccccc\0msg\0Bob\0<b@example.com>\x002026-01-01T00:00:00+00:00\n",
        );
        let tags = parse_tags(input).unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].kind, TagKind::Lightweight);
        assert_eq!(tags[1].kind, TagKind::Annotated);
        assert_eq!(tags[1].target, "ccccccc");
    }
}
