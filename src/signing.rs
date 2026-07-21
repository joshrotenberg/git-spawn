//! Typed inspection and configuration of commit and tag signing.
//!
//! Reached through [`Repository::signing`], which returns a [`SigningOps`]
//! handle:
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//! use git_spawn::signing::SignatureFormat;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Read the current signing configuration in one call.
//! let cfg = repo.signing().config().await?;
//! if cfg.sign_commits {
//!     println!("signing with key {:?} ({:?})", cfg.signing_key, cfg.format);
//! }
//!
//! // Configure SSH signing and turn it on for commits and tags.
//! repo.signing().set_format(SignatureFormat::Ssh).await?;
//! repo.signing().set_signing_key("~/.ssh/id_ed25519.pub").await?;
//! repo.signing().set_sign_commits(true).await?;
//! repo.signing().set_sign_tags(true).await?;
//! # Ok(())
//! # }
//! ```
//!
//! Every method is a thin, typed wrapper over `git config`, reading and
//! writing the four keys that govern signing: `user.signingkey`,
//! `gpg.format`, `commit.gpgsign`, and `tag.gpgsign`. Reads consult the
//! effective configuration (local, then global, then system), matching what
//! `git` itself would use; writes target the local repository config.
//!
//! These helpers only touch configuration. Producing an actual signature
//! still needs a real GPG or SSH key available to `git` at commit or tag
//! time, so a round-trip test of real signing must supply one of its own.

use crate::command::GitCommand;
use crate::command::config::ConfigCommand;
use crate::error::Result;
use crate::repo::Repository;

const KEY_SIGNING_KEY: &str = "user.signingkey";
const KEY_FORMAT: &str = "gpg.format";
const KEY_COMMIT_SIGN: &str = "commit.gpgsign";
const KEY_TAG_SIGN: &str = "tag.gpgsign";

/// The signature format `git` uses, as configured by `gpg.format`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SignatureFormat {
    /// OpenPGP / GnuPG (`openpgp`), git's default when unset.
    OpenPgp,
    /// SSH keys (`ssh`).
    Ssh,
    /// X.509 / S/MIME (`x509`).
    X509,
    /// Any other value git accepts, preserved verbatim.
    Other(String),
}

impl SignatureFormat {
    /// The string git stores in `gpg.format`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            SignatureFormat::OpenPgp => "openpgp",
            SignatureFormat::Ssh => "ssh",
            SignatureFormat::X509 => "x509",
            SignatureFormat::Other(s) => s,
        }
    }

    /// Classify a raw `gpg.format` value, mapping the known formats and
    /// keeping anything else as [`Other`](SignatureFormat::Other).
    fn from_value(value: &str) -> Self {
        match value {
            "openpgp" => SignatureFormat::OpenPgp,
            "ssh" => SignatureFormat::Ssh,
            "x509" => SignatureFormat::X509,
            other => SignatureFormat::Other(other.to_string()),
        }
    }
}

/// A snapshot of the four signing-related configuration values.
///
/// Produced by [`SigningOps::config`]. Absent `user.signingkey` and
/// `gpg.format` are `None`; the two boolean flags default to `false` when
/// unset, matching git's own behavior.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SigningConfig {
    /// `user.signingkey`, if set.
    pub signing_key: Option<String>,
    /// `gpg.format`, if set.
    pub format: Option<SignatureFormat>,
    /// `commit.gpgsign` (default `false`).
    pub sign_commits: bool,
    /// `tag.gpgsign` (default `false`).
    pub sign_tags: bool,
}

/// Operations on signing configuration, scoped to a [`Repository`].
///
/// Obtained via [`Repository::signing`]. The handle borrows the repository
/// for the duration of one chained call — there is no shared state.
#[derive(Debug)]
pub struct SigningOps<'a> {
    repo: &'a Repository,
}

impl<'a> SigningOps<'a> {
    /// The configured signing key (`user.signingkey`), or `None` if unset.
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails for a reason
    /// other than the key being absent.
    pub async fn signing_key(&self) -> Result<Option<String>> {
        self.get_opt(KEY_SIGNING_KEY).await
    }

    /// The configured signature format (`gpg.format`), or `None` if unset.
    ///
    /// A `None` result means git falls back to its default of OpenPGP.
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails for a reason
    /// other than the key being absent.
    pub async fn format(&self) -> Result<Option<SignatureFormat>> {
        Ok(self
            .get_opt(KEY_FORMAT)
            .await?
            .map(|v| SignatureFormat::from_value(&v)))
    }

    /// Whether commits are signed by default (`commit.gpgsign`).
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails for a reason
    /// other than the key being absent.
    pub async fn sign_commits(&self) -> Result<bool> {
        self.get_bool(KEY_COMMIT_SIGN).await
    }

    /// Whether tags are signed by default (`tag.gpgsign`).
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails for a reason
    /// other than the key being absent.
    pub async fn sign_tags(&self) -> Result<bool> {
        self.get_bool(KEY_TAG_SIGN).await
    }

    /// Read all four signing values in one [`SigningConfig`].
    ///
    /// # Errors
    /// Returns an error if any of the underlying `git config` reads fails.
    pub async fn config(&self) -> Result<SigningConfig> {
        Ok(SigningConfig {
            signing_key: self.signing_key().await?,
            format: self.format().await?,
            sign_commits: self.sign_commits().await?,
            sign_tags: self.sign_tags().await?,
        })
    }

    /// Set the signing key (`user.signingkey`).
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails.
    pub async fn set_signing_key(&self, key: impl Into<String>) -> Result<()> {
        self.set(KEY_SIGNING_KEY, key.into()).await
    }

    /// Set the signature format (`gpg.format`).
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails.
    pub async fn set_format(&self, format: SignatureFormat) -> Result<()> {
        self.set(KEY_FORMAT, format.as_str().to_string()).await
    }

    /// Turn default commit signing on or off (`commit.gpgsign`).
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails.
    pub async fn set_sign_commits(&self, on: bool) -> Result<()> {
        self.set(KEY_COMMIT_SIGN, bool_value(on)).await
    }

    /// Turn default tag signing on or off (`tag.gpgsign`).
    ///
    /// # Errors
    /// Returns an error if the `git config` invocation fails.
    pub async fn set_sign_tags(&self, on: bool) -> Result<()> {
        self.set(KEY_TAG_SIGN, bool_value(on)).await
    }

    /// `git config <key>`, treating a missing key as `None`.
    async fn get_opt(&self, key: &str) -> Result<Option<String>> {
        self.repo
            .config(ConfigCommand::get(key))
            .execute_value_opt()
            .await
    }

    /// `git config <key>` parsed as a boolean, defaulting to `false`.
    async fn get_bool(&self, key: &str) -> Result<bool> {
        Ok(self.get_opt(key).await?.as_deref().is_some_and(parse_bool))
    }

    /// `git config <key> <value>` in the local repository config.
    async fn set(&self, key: &str, value: String) -> Result<()> {
        self.repo
            .config(ConfigCommand::set(key, value))
            .execute()
            .await?;
        Ok(())
    }
}

impl Repository {
    /// Operations on commit and tag signing configuration.
    #[must_use]
    pub fn signing(&self) -> SigningOps<'_> {
        SigningOps { repo: self }
    }
}

/// The canonical string for a boolean git config value.
fn bool_value(on: bool) -> String {
    if on { "true" } else { "false" }.to_string()
}

/// Interpret a raw git config value as a boolean.
///
/// Follows git's own truthiness rules: `true`, `yes`, `on`, `1`, and an empty
/// value (a bare `[section] key` line) are true; everything else is false.
/// Matching is case-insensitive and ignores surrounding whitespace.
fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "on" | "1" | ""
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_format_round_trips_known_values() {
        for (raw, fmt) in [
            ("openpgp", SignatureFormat::OpenPgp),
            ("ssh", SignatureFormat::Ssh),
            ("x509", SignatureFormat::X509),
        ] {
            assert_eq!(SignatureFormat::from_value(raw), fmt);
            assert_eq!(fmt.as_str(), raw);
        }
    }

    #[test]
    fn signature_format_preserves_unknown_values() {
        let fmt = SignatureFormat::from_value("smime");
        assert_eq!(fmt, SignatureFormat::Other("smime".to_string()));
        assert_eq!(fmt.as_str(), "smime");
    }

    #[test]
    fn parse_bool_matches_git_truthiness() {
        for truthy in ["true", "TRUE", "yes", "On", "1", "", "  true  "] {
            assert!(parse_bool(truthy), "{truthy:?} should be true");
        }
        for falsy in ["false", "no", "off", "0", "nope"] {
            assert!(!parse_bool(falsy), "{falsy:?} should be false");
        }
    }

    #[test]
    fn bool_value_is_canonical() {
        assert_eq!(bool_value(true), "true");
        assert_eq!(bool_value(false), "false");
    }
}
