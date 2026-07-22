//! Typed management of repository hooks.
//!
//! Reached through [`Repository::hooks`], which returns a [`HookOps`] handle:
//!
//! ```no_run
//! # async fn ex() -> git_spawn::Result<()> {
//! use git_spawn::Repository;
//!
//! let repo = Repository::open("/path/to/repo")?;
//!
//! // Every hook file, with whether git would run it.
//! for hook in repo.hooks().list().await? {
//!     println!("{}  {}", hook.name, if hook.enabled { "enabled" } else { "sample only" });
//! }
//!
//! repo.hooks().install("pre-commit", "#!/bin/sh\nexit 0\n").await?;
//! repo.hooks().disable("pre-commit").await?;
//! repo.hooks().enable("pre-commit").await?;
//! repo.hooks().remove("pre-commit").await?;
//! # Ok(())
//! # }
//! ```
//!
//! These helpers touch the hooks directory directly rather than shelling out
//! to git. The directory is `<git-dir>/hooks` unless `core.hooksPath` is set,
//! in which case that path is honored (resolved against the working tree when
//! relative), matching where git itself looks for hooks.
//!
//! A hook is [`enabled`](Hook::enabled) when a non-sample file exists at its
//! name and is executable. Git ships samples as `<name>.sample`; those are
//! reported under their bare name with `enabled == false`. Because the
//! executable bit is a Unix concept, [`enable`](HookOps::enable) and
//! [`disable`](HookOps::disable) adjust that bit on Unix and are no-ops
//! elsewhere, and on non-Unix platforms any non-sample hook file is treated
//! as enabled.

use std::collections::BTreeMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};

use crate::command::config::ConfigCommand;
use crate::error::Result;
use crate::repo::Repository;

/// The suffix git uses for its shipped sample hooks.
const SAMPLE_SUFFIX: &str = ".sample";

/// One hook in the repository's hooks directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Hook {
    /// Hook name (e.g. `"pre-commit"`), with any `.sample` suffix stripped.
    pub name: String,
    /// Full path to the backing file.
    pub path: PathBuf,
    /// Whether git would run this hook: a non-sample, executable file.
    pub enabled: bool,
}

/// Operations on repository hooks, scoped to a [`Repository`].
///
/// Obtained via [`Repository::hooks`]. The handle borrows the repository for
/// the duration of one chained call — there is no shared state.
#[derive(Debug)]
pub struct HookOps<'a> {
    repo: &'a Repository,
}

impl<'a> HookOps<'a> {
    /// List every hook file in the hooks directory.
    ///
    /// Samples (`<name>.sample`) are reported under their bare name with
    /// `enabled == false`; a real hook of the same name takes precedence over
    /// its sample. Records are sorted by name. A missing hooks directory
    /// yields an empty list.
    ///
    /// # Errors
    /// Returns an error if `core.hooksPath` cannot be read or the hooks
    /// directory cannot be listed for a reason other than not existing.
    pub async fn list(&self) -> Result<Vec<Hook>> {
        let dir = self.hooks_dir().await?;
        let mut entries = Vec::new();
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        while let Some(entry) = rd.next_entry().await? {
            let meta = entry.metadata().await?;
            if !meta.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            entries.push((name, is_executable(&meta)));
        }
        Ok(classify_hooks(&dir, entries))
    }

    /// Install a hook named `name` with the given `body`, making it
    /// executable (on Unix). Creates the hooks directory if it is missing and
    /// overwrites any existing file at that name.
    ///
    /// # Errors
    /// Returns an error if the hooks directory cannot be created or the file
    /// cannot be written.
    pub async fn install(&self, name: impl AsRef<str>, body: impl AsRef<[u8]>) -> Result<()> {
        let dir = self.hooks_dir().await?;
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(name.as_ref());
        tokio::fs::write(&path, body.as_ref()).await?;
        set_executable(&path).await?;
        Ok(())
    }

    /// Remove the hook named `name`.
    ///
    /// # Errors
    /// Returns an error if no such hook file exists or it cannot be deleted.
    pub async fn remove(&self, name: impl AsRef<str>) -> Result<()> {
        let path = self.hooks_dir().await?.join(name.as_ref());
        tokio::fs::remove_file(&path).await?;
        Ok(())
    }

    /// Enable the hook named `name` by making its file executable.
    ///
    /// On non-Unix platforms this only checks that the file exists, since the
    /// executable bit does not gate hook execution there.
    ///
    /// # Errors
    /// Returns an error if no such hook file exists or its mode cannot be
    /// changed.
    pub async fn enable(&self, name: impl AsRef<str>) -> Result<()> {
        let path = self.hooks_dir().await?.join(name.as_ref());
        set_executable(&path).await?;
        Ok(())
    }

    /// Disable the hook named `name` by clearing the executable bits on its
    /// file (Unix only; a no-op that still checks existence elsewhere).
    ///
    /// # Errors
    /// Returns an error if no such hook file exists or its mode cannot be
    /// changed.
    pub async fn disable(&self, name: impl AsRef<str>) -> Result<()> {
        let path = self.hooks_dir().await?.join(name.as_ref());
        clear_executable(&path).await?;
        Ok(())
    }

    /// Resolve the hooks directory, honoring `core.hooksPath`.
    async fn hooks_dir(&self) -> Result<PathBuf> {
        if let Some(custom) = self
            .repo
            .config(ConfigCommand::get("core.hooksPath"))
            .execute_value_opt()
            .await?
        {
            let p = PathBuf::from(custom);
            return Ok(if p.is_absolute() {
                p
            } else {
                self.repo.path().join(p)
            });
        }
        Ok(self.repo.git_dir().join("hooks"))
    }
}

impl Repository {
    /// Operations on repository hooks.
    #[must_use]
    pub fn hooks(&self) -> HookOps<'_> {
        HookOps { repo: self }
    }
}

/// Turn a directory listing into one [`Hook`] per hook name.
///
/// Real (non-sample) files are recorded first; a sample only contributes when
/// no real file of that name is present. Output is sorted by name.
fn classify_hooks(dir: &Path, entries: Vec<(String, bool)>) -> Vec<Hook> {
    let mut map: BTreeMap<String, Hook> = BTreeMap::new();
    for (file, exec) in entries.iter().filter(|(f, _)| !f.ends_with(SAMPLE_SUFFIX)) {
        map.insert(
            file.clone(),
            Hook {
                name: file.clone(),
                path: dir.join(file),
                enabled: *exec,
            },
        );
    }
    for (file, _) in entries.iter().filter(|(f, _)| f.ends_with(SAMPLE_SUFFIX)) {
        let name = file
            .strip_suffix(SAMPLE_SUFFIX)
            .expect("filtered to sample suffix")
            .to_string();
        map.entry(name.clone()).or_insert_with(|| Hook {
            name,
            path: dir.join(file),
            enabled: false,
        });
    }
    map.into_values().collect()
}

/// Whether a file's metadata marks it executable.
///
/// On Unix this checks the owner/group/other execute bits; elsewhere the
/// executable bit does not gate hook execution, so any file counts.
#[cfg(unix)]
fn is_executable(meta: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_meta: &Metadata) -> bool {
    true
}

/// Set the executable bits on `path` (Unix), or verify it exists (elsewhere).
#[cfg(unix)]
async fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(path).await?.permissions();
    perms.set_mode(perms.mode() | 0o755);
    tokio::fs::set_permissions(path, perms).await?;
    Ok(())
}

#[cfg(not(unix))]
async fn set_executable(path: &Path) -> Result<()> {
    tokio::fs::metadata(path).await?;
    Ok(())
}

/// Clear the executable bits on `path` (Unix), or verify it exists (elsewhere).
#[cfg(unix)]
async fn clear_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = tokio::fs::metadata(path).await?.permissions();
    perms.set_mode(perms.mode() & !0o111);
    tokio::fs::set_permissions(path, perms).await?;
    Ok(())
}

#[cfg(not(unix))]
async fn clear_executable(path: &Path) -> Result<()> {
    tokio::fs::metadata(path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(hooks: &[Hook]) -> Vec<&str> {
        hooks.iter().map(|h| h.name.as_str()).collect()
    }

    #[test]
    fn empty_listing_is_empty() {
        assert!(classify_hooks(Path::new("/h"), Vec::new()).is_empty());
    }

    #[test]
    fn executable_real_hook_is_enabled() {
        let hooks = classify_hooks(Path::new("/h"), vec![("pre-commit".into(), true)]);
        assert_eq!(names(&hooks), vec!["pre-commit"]);
        assert!(hooks[0].enabled);
        assert_eq!(hooks[0].path, Path::new("/h/pre-commit"));
    }

    #[test]
    fn non_executable_real_hook_is_disabled() {
        let hooks = classify_hooks(Path::new("/h"), vec![("pre-commit".into(), false)]);
        assert!(!hooks[0].enabled);
    }

    #[test]
    fn sample_is_reported_under_bare_name_disabled() {
        let hooks = classify_hooks(Path::new("/h"), vec![("pre-commit.sample".into(), true)]);
        assert_eq!(names(&hooks), vec!["pre-commit"]);
        assert!(!hooks[0].enabled, "a sample is never enabled");
        assert_eq!(hooks[0].path, Path::new("/h/pre-commit.sample"));
    }

    #[test]
    fn real_hook_takes_precedence_over_its_sample() {
        let hooks = classify_hooks(
            Path::new("/h"),
            vec![
                ("pre-commit.sample".into(), true),
                ("pre-commit".into(), true),
            ],
        );
        assert_eq!(names(&hooks), vec!["pre-commit"]);
        assert!(hooks[0].enabled);
        assert_eq!(hooks[0].path, Path::new("/h/pre-commit"));
    }

    #[test]
    fn results_are_sorted_by_name() {
        let hooks = classify_hooks(
            Path::new("/h"),
            vec![
                ("pre-push".into(), true),
                ("commit-msg.sample".into(), true),
                ("pre-commit".into(), false),
            ],
        );
        assert_eq!(names(&hooks), vec!["commit-msg", "pre-commit", "pre-push"]);
    }
}
