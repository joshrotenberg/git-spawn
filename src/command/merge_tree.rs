//! `git merge-tree` — perform a trial three-way merge.
use crate::command::{CommandExecutor, CommandOutput, GitCommand};
use crate::error::{Error, Result};
use async_trait::async_trait;

/// Typed result of `git merge-tree --write-tree`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct MergeTreeResult {
    /// Object ID of the tree produced by the trial merge.
    pub tree: String,
    /// Whether the trial merge completed without conflicts.
    pub clean: bool,
    /// Paths with conflicts, in Git's reported order.
    pub conflicts: Vec<String>,
}

/// Builder for `git merge-tree`.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MergeTreeCommand {
    /// Shared executor.
    pub executor: CommandExecutor,
    /// Explicit merge base.
    ///
    /// The typed write-tree form renders this as `--merge-base=<tree-ish>`;
    /// the legacy three-tree form renders it as the first positional value.
    pub base: Option<String>,
    /// First tree or branch.
    pub ours: Option<String>,
    /// Second tree or branch.
    pub theirs: Option<String>,
    /// Write a real tree object and emit machine-readable merge information.
    pub write_tree: bool,
    /// Emit NUL-delimited output.
    pub null_terminate: bool,
    /// Show messages only.
    pub messages: bool,
}
impl MergeTreeCommand {
    /// Create a command.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Set an explicit merge base.
    pub fn base(&mut self, v: impl Into<String>) -> &mut Self {
        self.base = Some(v.into());
        self
    }
    /// Set the first side.
    pub fn ours(&mut self, v: impl Into<String>) -> &mut Self {
        self.ours = Some(v.into());
        self
    }
    /// Set the second side.
    pub fn theirs(&mut self, v: impl Into<String>) -> &mut Self {
        self.theirs = Some(v.into());
        self
    }
    /// Enable `--write-tree` and return a typed [`MergeTreeResult`].
    ///
    /// Typed execution also passes `--name-only` so conflicted paths can be
    /// decoded without exposing Git's index-stage tuples.
    pub fn write_tree(&mut self) -> &mut Self {
        self.write_tree = true;
        self
    }
    /// Enable `-z`.
    pub fn null_terminate(&mut self) -> &mut Self {
        self.null_terminate = true;
        self
    }
    /// Enable `--messages`.
    pub fn messages(&mut self) -> &mut Self {
        self.messages = true;
        self
    }
}
#[async_trait]
impl GitCommand for MergeTreeCommand {
    type Output = MergeTreeResult;
    fn get_executor(&self) -> &CommandExecutor {
        &self.executor
    }
    fn get_executor_mut(&mut self) -> &mut CommandExecutor {
        &mut self.executor
    }
    fn build_command_args(&self) -> Vec<String> {
        let mut a = vec!["merge-tree".into()];
        if self.write_tree {
            a.push("--write-tree".into());
            a.push("--name-only".into());
        }
        if self.null_terminate {
            a.push("-z".into())
        }
        if self.messages {
            a.push("--messages".into())
        }
        if let Some(v) = &self.base {
            if self.write_tree {
                a.push(format!("--merge-base={v}"));
            } else {
                a.push(v.clone());
            }
        }
        if let Some(v) = &self.ours {
            a.push(v.clone())
        }
        if let Some(v) = &self.theirs {
            a.push(v.clone())
        }
        a
    }
    async fn execute(&self) -> Result<MergeTreeResult> {
        if !self.write_tree {
            return Err(Error::invalid_config(
                "typed merge-tree execution requires --write-tree; use execute_raw for the legacy form",
            ));
        }

        // `--write-tree` exits 1 for a completed merge with conflicts while
        // still emitting the result tree and conflict details.
        let output = self
            .executor
            .execute_command_os_checked_by(self.build_command_os_args(), |output| {
                output.exit_code == 0
                    || (output.exit_code == 1 && starts_with_object_id(&output.stdout))
            })
            .await?;
        parse_result(output, self.null_terminate)
    }
}

fn starts_with_object_id(stdout: &[u8]) -> bool {
    let end = stdout
        .iter()
        .position(|byte| matches!(*byte, b'\n' | b'\0'))
        .unwrap_or(stdout.len());
    is_object_id(&stdout[..end])
}

fn is_object_id(value: &[u8]) -> bool {
    matches!(value.len(), 40 | 64) && value.iter().all(u8::is_ascii_hexdigit)
}

fn parse_result(output: CommandOutput, null_terminate: bool) -> Result<MergeTreeResult> {
    let delimiter = if null_terminate { b'\0' } else { b'\n' };
    let mut fields = output.stdout.split(|byte| *byte == delimiter);
    let tree = fields.next().unwrap_or_default();
    if !is_object_id(tree) {
        return Err(Error::parse_error(
            "merge-tree output did not begin with a tree object ID",
        ));
    }

    let conflicts = if output.exit_code == 1 {
        fields
            .take_while(|field| !field.is_empty())
            .map(|field| String::from_utf8_lossy(field).into_owned())
            .collect()
    } else {
        Vec::new()
    };

    Ok(MergeTreeResult {
        tree: String::from_utf8_lossy(tree).into_owned(),
        clean: output.exit_code == 0,
        conflicts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TREE: &str = "0123456789abcdef0123456789abcdef01234567";

    fn output(stdout: Vec<u8>, exit_code: i32) -> CommandOutput {
        CommandOutput {
            stdout,
            stderr: String::new(),
            exit_code,
            success: exit_code == 0,
        }
    }

    #[test]
    fn recognizes_newline_and_nul_terminated_object_ids() {
        assert!(starts_with_object_id(format!("{TREE}\n").as_bytes()));
        assert!(starts_with_object_id(format!("{TREE}\0path\0").as_bytes()));
        assert!(!starts_with_object_id(b"not-an-object-id\n"));
    }

    #[test]
    fn parses_clean_result() {
        let result = parse_result(output(format!("{TREE}\n").into_bytes(), 0), false).unwrap();
        assert_eq!(result.tree, TREE);
        assert!(result.clean);
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn parses_conflicted_newline_result() {
        let stdout = format!(
            "{TREE}\nfile.txt\ndir/other.txt\n\nCONFLICT (content): Merge conflict in file.txt\n"
        );
        let result = parse_result(output(stdout.into_bytes(), 1), false).unwrap();
        assert_eq!(result.tree, TREE);
        assert!(!result.clean);
        assert_eq!(result.conflicts, ["file.txt", "dir/other.txt"]);
    }

    #[test]
    fn parses_conflicted_nul_result() {
        let stdout = format!(
            "{TREE}\0file.txt\0dir/other.txt\0\01\0file.txt\0CONFLICT (content)\0message\0"
        );
        let result = parse_result(output(stdout.into_bytes(), 1), true).unwrap();
        assert_eq!(result.tree, TREE);
        assert!(!result.clean);
        assert_eq!(result.conflicts, ["file.txt", "dir/other.txt"]);
    }
}
