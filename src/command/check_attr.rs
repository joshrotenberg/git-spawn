//! `git check-attr` — report gitattributes for paths.

use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git check-attr`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct CheckAttrCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Attribute names to inspect.
    pub attributes: Vec<String>,
    /// Paths whose attributes should be reported.
    pub paths: Vec<String>,
    /// `--all`: report every attribute associated with the paths.
    pub all: bool,
    /// `--cached`: consult only attributes from the index.
    pub cached: bool,
}

impl CheckAttrCommand {
    /// New command with no attributes or paths yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an attribute name to inspect.
    pub fn attribute(&mut self, attribute: impl Into<String>) -> &mut Self {
        self.attributes.push(attribute.into());
        self
    }

    /// Add several attribute names.
    pub fn attributes<I, S>(&mut self, attributes: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.attributes
            .extend(attributes.into_iter().map(Into::into));
        self
    }

    /// Add a path to inspect.
    pub fn path(&mut self, path: impl Into<String>) -> &mut Self {
        self.paths.push(path.into());
        self
    }

    /// Add several paths, preserving their order.
    pub fn paths<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.paths.extend(paths.into_iter().map(Into::into));
        self
    }

    /// Report every attribute associated with each path (`--all`).
    pub fn all(&mut self) -> &mut Self {
        self.all = true;
        self
    }

    /// Consult only the attributes in the index (`--cached`).
    pub fn cached(&mut self) -> &mut Self {
        self.cached = true;
        self
    }

    fn validate(&self) -> Result<()> {
        if self.paths.is_empty() {
            return Err(Error::invalid_config(
                "check-attr: at least one path is required",
            ));
        }
        if self.all == !self.attributes.is_empty() {
            return Err(Error::invalid_config(
                "check-attr: select either --all or at least one attribute",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl GitCommand for CheckAttrCommand {
    type Output = CommandOutput;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["check-attr".to_string()];
        if self.all {
            args.push("--all".into());
        }
        if self.cached {
            args.push("--cached".into());
        }
        args.extend(self.attributes.iter().cloned());
        if !self.paths.is_empty() {
            args.push("--".into());
            args.extend(self.paths.iter().cloned());
        }
        args
    }

    async fn execute(&self) -> Result<CommandOutput> {
        self.validate()?;
        self.execute_raw().await
    }
}
