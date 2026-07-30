//! `git var` — print a git logical variable.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git var`.
///
/// Logical variables are the values git computes for itself rather than reads
/// straight from config: the editor it would open, the pager it would pipe
/// through, the identity it would stamp on a commit. Two shapes:
///
/// - [`get`](Self::get), `git var <name>` — one variable, one line of output;
/// - [`list`](Self::list), `git var -l` — every logical variable followed by
///   the whole effective config, as `key=value` lines.
///
/// git accepts a name or `-l` but not both and not neither, so
/// [`execute`](GitCommand::execute) checks that rather than letting git fail on
/// a usage error (exit 129).
///
/// Names are git's, not this crate's: `GIT_AUTHOR_IDENT`,
/// `GIT_COMMITTER_IDENT`, `GIT_EDITOR`, `GIT_PAGER`, and the newer
/// `GIT_DEFAULT_BRANCH` and `GIT_SEQUENCE_EDITOR`. Which ones exist depends on
/// the git version, and an unknown name is a `usage:` failure rather than an
/// empty value.
///
/// No parser: [`execute`](GitCommand::execute) returns trimmed stdout, a single
/// value under [`get`](Self::get) and newline-separated `key=value` lines under
/// [`list`](Self::list).
#[derive(Debug, Clone)]
pub struct VarCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// The variable to print, when reading one.
    pub name: Option<String>,
    /// `-l`: list every logical variable and the effective config.
    pub list: bool,
}

impl VarCommand {
    /// Print one logical variable, e.g. `get("GIT_AUTHOR_IDENT")`.
    pub fn get(name: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            name: Some(name.into()),
            list: false,
        }
    }

    /// List every logical variable and the effective config (`-l`).
    #[must_use]
    pub fn list() -> Self {
        Self {
            executor: CommandExecutor::default(),
            name: None,
            list: true,
        }
    }

    /// Check that exactly one of a name and `-l` is selected.
    ///
    /// The constructors pick one, but the fields are public and can be set
    /// afterwards, so both and neither are reachable.
    fn validate(&self) -> Result<()> {
        match (&self.name, self.list) {
            (Some(_), true) => Err(Error::invalid_config(
                "var: a variable name and -l cannot be used together",
            )),
            (None, false) => Err(Error::invalid_config("var requires a variable name or -l")),
            (Some(name), false) if name.is_empty() => {
                Err(Error::invalid_config("var requires a variable name"))
            }
            _ => Ok(()),
        }
    }
}

#[async_trait]
impl GitCommand for VarCommand {
    /// Trimmed stdout — the value for a name, `key=value` lines for `-l`.
    type Output = String;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["var".to_string()];
        if self.list {
            args.push("-l".into());
        }
        if let Some(name) = &self.name {
            args.push(name.clone());
        }
        args
    }

    async fn execute(&self) -> Result<String> {
        self.validate()?;
        let out = self.execute_raw().await?;
        Ok(out.stdout_trimmed())
    }
}
