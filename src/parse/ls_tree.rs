//! Parser for `git ls-tree` output.
//!
//! Each entry is `<mode> SP <type> SP <sha>[ SP <size>] TAB <path>`. The
//! `<size>` field is only present when `-l`/`--long` was requested, and is
//! `-` for non-blob entries. `--name-only` output has a different shape (a
//! bare newline-separated path list with no tab), so it gets its own helper.

use crate::error::{Error, Result};

/// Type of a tree entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TreeObjectType {
    /// A blob (file).
    Blob,
    /// A tree (directory).
    Tree,
    /// A commit (submodule / gitlink).
    Commit,
    /// Some other type not recognized.
    Other(String),
}

impl From<&str> for TreeObjectType {
    fn from(s: &str) -> Self {
        match s {
            "blob" => Self::Blob,
            "tree" => Self::Tree,
            "commit" => Self::Commit,
            other => Self::Other(other.to_string()),
        }
    }
}

/// One parsed entry from `git ls-tree`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TreeEntry {
    /// File mode (e.g. `100644`).
    pub mode: String,
    /// Object type.
    pub object_type: TreeObjectType,
    /// Object SHA.
    pub sha: String,
    /// Object size in bytes; populated only when `-l`/`--long` was
    /// requested. The `-` sentinel (non-blob entries) maps to `None`.
    pub size: Option<u64>,
    /// Path relative to the tree root.
    pub path: String,
}

/// Parse the output of `git ls-tree` (with or without `-l`/`--long`).
///
/// # Errors
/// Returns [`Error::ParseError`] if a line is malformed.
///
/// # Example
/// ```
/// use git_spawn::parse::{parse_ls_tree, TreeObjectType};
/// let input = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tfoo.txt\n\
///              040000 tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\tsubdir\n";
/// let entries = parse_ls_tree(input).unwrap();
/// assert_eq!(entries.len(), 2);
/// assert_eq!(entries[0].object_type, TreeObjectType::Blob);
/// assert_eq!(entries[0].path, "foo.txt");
/// assert_eq!(entries[1].object_type, TreeObjectType::Tree);
/// ```
pub fn parse_ls_tree(input: &str) -> Result<Vec<TreeEntry>> {
    let mut out = Vec::new();
    for line in input.lines() {
        if line.is_empty() {
            continue;
        }
        let mut halves = line.splitn(2, '\t');
        let metadata = halves
            .next()
            .ok_or_else(|| Error::parse_error("ls-tree entry missing metadata"))?;
        let path = halves
            .next()
            .ok_or_else(|| Error::parse_error("ls-tree entry missing path"))?;

        let mut fields = metadata.split_whitespace();
        let mode = fields
            .next()
            .ok_or_else(|| Error::parse_error("ls-tree entry missing mode"))?
            .to_string();
        let object_type = TreeObjectType::from(
            fields
                .next()
                .ok_or_else(|| Error::parse_error("ls-tree entry missing type"))?,
        );
        let sha = fields
            .next()
            .ok_or_else(|| Error::parse_error("ls-tree entry missing sha"))?
            .to_string();
        let size = match fields.next() {
            None | Some("-") => None,
            Some(s) => Some(
                s.parse::<u64>()
                    .map_err(|_| Error::parse_error("ls-tree entry has invalid size"))?,
            ),
        };

        out.push(TreeEntry {
            mode,
            object_type,
            sha,
            size,
            path: path.to_string(),
        });
    }
    Ok(out)
}

/// Parse the output of `git ls-tree --name-only`: a bare newline-separated
/// list of paths.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_ls_tree_name_only;
/// let input = "foo.txt\nsubdir\n";
/// assert_eq!(parse_ls_tree_name_only(input), vec!["foo.txt", "subdir"]);
/// ```
pub fn parse_ls_tree_name_only(input: &str) -> Vec<String> {
    input
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_blob_entry() {
        let input = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tfoo.txt\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].mode, "100644");
        assert_eq!(entries[0].object_type, TreeObjectType::Blob);
        assert_eq!(entries[0].sha, "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
        assert_eq!(entries[0].size, None);
        assert_eq!(entries[0].path, "foo.txt");
    }

    #[test]
    fn parses_tree_entry() {
        let input = "040000 tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\tsubdir\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(entries[0].object_type, TreeObjectType::Tree);
    }

    #[test]
    fn parses_commit_entry() {
        let input = "160000 commit e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tsubmodule\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(entries[0].object_type, TreeObjectType::Commit);
    }

    #[test]
    fn parses_other_type() {
        let input = "160000 gitlink e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tstrange\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(
            entries[0].object_type,
            TreeObjectType::Other("gitlink".to_string())
        );
    }

    #[test]
    fn parses_long_format_with_size() {
        let input = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391      12\tfoo.txt\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(entries[0].size, Some(12));
    }

    #[test]
    fn parses_long_format_dash_sentinel_for_non_blob() {
        let input = "040000 tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904       -\tsubdir\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(entries[0].size, None);
    }

    #[test]
    fn preserves_tab_containing_paths() {
        let input = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tweird\tname.txt\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(entries[0].path, "weird\tname.txt");
    }

    #[test]
    fn multiple_entries() {
        let input = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tfoo.txt\n\
                      040000 tree 4b825dc642cb6eb9a060e54bf8d69288fbee4904\tsubdir\n";
        let entries = parse_ls_tree(input).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn empty_input_yields_no_entries() {
        assert!(parse_ls_tree("").unwrap().is_empty());
    }

    #[test]
    fn malformed_missing_path_errors() {
        let input = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\n";
        assert!(parse_ls_tree(input).is_err());
    }

    #[test]
    fn name_only_parses_bare_paths() {
        let input = "foo.txt\nsubdir\n";
        assert_eq!(parse_ls_tree_name_only(input), vec!["foo.txt", "subdir"]);
    }

    #[test]
    fn name_only_empty_input_yields_empty_vec() {
        assert!(parse_ls_tree_name_only("").is_empty());
    }
}
