//! `git check-ref-format` — validate a reference name.

use crate::command::{CommandExecutor, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Builder for `git check-ref-format`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CheckRefFormatCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Ref or branch name to validate.
    pub name: String,
    /// `--branch`: validate the shorter branch-name form.
    pub branch: bool,
    /// `--allow-onelevel`: permit a ref with no slash.
    pub allow_onelevel: bool,
    /// `--refspec-pattern`: permit one `*` wildcard.
    pub refspec_pattern: bool,
    /// `--normalize`: normalize redundant slashes before validating.
    pub normalize: bool,
}

impl CheckRefFormatCommand {
    /// Validate a fully qualified ref name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            executor: CommandExecutor::default(),
            name: name.into(),
            branch: false,
            allow_onelevel: false,
            refspec_pattern: false,
            normalize: false,
        }
    }

    /// Validate a branch name (`--branch`).
    pub fn branch(name: impl Into<String>) -> Self {
        let mut command = Self::new(name);
        command.branch = true;
        command
    }

    /// Permit a one-level ref name.
    pub fn allow_onelevel(&mut self) -> &mut Self {
        self.allow_onelevel = true;
        self
    }

    /// Permit a single wildcard for a refspec pattern.
    pub fn refspec_pattern(&mut self) -> &mut Self {
        self.refspec_pattern = true;
        self
    }

    /// Normalize redundant slashes and print the normalized ref.
    pub fn normalize(&mut self) -> &mut Self {
        self.normalize = true;
        self
    }

    /// Return `false` for an invalid name while preserving spawning failures.
    ///
    /// Git reports an invalid full ref with status 1, but an invalid name in
    /// `--branch` mode with status 128, so both are ordinary negative results.
    pub async fn is_valid(&self) -> Result<bool> {
        // In full-ref mode Git parses a leading `-` as an option (for example,
        // `--help` exits with usage status 129) rather than as the ref name we
        // were asked to validate. Such names cannot be valid refs, so reject
        // them without spawning Git and keep this method a reliable predicate.
        if !self.branch && self.name.starts_with('-') {
            return Ok(false);
        }
        self.validate()?;
        match self.execute_raw().await {
            Ok(_) => Ok(true),
            Err(Error::CommandFailed {
                exit_code: 1 | 128, ..
            }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(Error::invalid_config(
                "check-ref-format: a name is required",
            ));
        }
        if !self.branch && self.name.starts_with('-') {
            return Err(Error::invalid_config(
                "check-ref-format: a full ref name cannot begin with '-'",
            ));
        }
        if self.branch && (self.allow_onelevel || self.refspec_pattern || self.normalize) {
            return Err(Error::invalid_config(
                "check-ref-format: --branch cannot be combined with ref-format options",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl GitCommand for CheckRefFormatCommand {
    /// Normalized ref under `--normalize`; otherwise the empty string.
    type Output = String;

    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }

    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }

    fn build_command_args(&self) -> Vec<String> {
        let mut args = vec!["check-ref-format".to_string()];
        if self.branch {
            args.push("--branch".into());
        } else {
            if self.allow_onelevel {
                args.push("--allow-onelevel".into());
            }
            if self.refspec_pattern {
                args.push("--refspec-pattern".into());
            }
            if self.normalize {
                args.push("--normalize".into());
            }
        }
        args.push(self.name.clone());
        args
    }

    async fn execute(&self) -> Result<String> {
        self.validate()?;
        Ok(self.execute_raw().await?.stdout_trimmed())
    }
}
