//! Parser for `git --version`.
//!
//! The output is a single `git version <version>` line. The version itself is
//! dotted decimal, but only the first two components are reliably numeric:
//! distributions append their own trailing components (`2.45.1.windows.1`) or
//! a parenthesized build tag after a space (`2.39.5 (Apple Git-154)`). The
//! parser takes the leading numeric components as major/minor/patch, keeps any
//! trailing dotted components as [`GitVersion::suffix`], and preserves the
//! whole version text in [`GitVersion::raw`].
//!
//! `--build-options` output is accepted too: the `git version` line is found
//! among the lines and the build's `cpu:`/`sizeof-*` lines are ignored.

/// A parsed `git --version` line.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GitVersion {
    /// The first component, e.g. `2` in `2.45.1`.
    pub major: u32,
    /// The second component, e.g. `45` in `2.45.1`.
    pub minor: u32,
    /// The third component when it is numeric, e.g. `1` in `2.45.1`.
    pub patch: Option<u32>,
    /// Trailing dotted components past the numeric ones, e.g. `windows.1` in
    /// `2.45.1.windows.1`.
    pub suffix: Option<String>,
    /// The version text as git printed it, including any build tag that
    /// followed a space: `2.39.5 (Apple Git-154)`.
    pub raw: String,
}

impl GitVersion {
    /// Whether this version is at least `major.minor.patch`.
    ///
    /// A missing [`patch`](Self::patch) counts as `0`, and the
    /// [`suffix`](Self::suffix) is ignored: a distribution's trailing
    /// components say nothing about upstream feature availability.
    ///
    /// # Example
    /// ```
    /// use git_spawn::parse::parse_version;
    /// let v = parse_version("git version 2.45.1\n").unwrap();
    /// assert!(v.is_at_least(2, 45, 0));
    /// assert!(!v.is_at_least(2, 46, 0));
    /// ```
    #[must_use]
    pub fn is_at_least(&self, major: u32, minor: u32, patch: u32) -> bool {
        (self.major, self.minor, self.patch.unwrap_or(0)) >= (major, minor, patch)
    }
}

/// Parse the output of `git --version`.
///
/// Returns [`None`] when no `git version` line is present, or when the two
/// leading components are not numeric.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_version;
/// let v = parse_version("git version 2.39.5 (Apple Git-154)\n").unwrap();
/// assert_eq!((v.major, v.minor, v.patch), (2, 39, Some(5)));
/// assert_eq!(v.suffix, None);
/// assert_eq!(v.raw, "2.39.5 (Apple Git-154)");
/// ```
#[must_use]
pub fn parse_version(input: &str) -> Option<GitVersion> {
    let raw = input
        .lines()
        .find_map(|line| line.trim().strip_prefix("git version "))?
        .trim();
    let token = raw.split_whitespace().next()?;

    let mut components = token.split('.');
    let major = components.next()?.parse::<u32>().ok()?;
    let minor = components.next()?.parse::<u32>().ok()?;

    let rest: Vec<&str> = components.collect();
    let (patch, suffix_parts) = match rest.split_first() {
        Some((first, tail)) => match first.parse::<u32>() {
            Ok(patch) => (Some(patch), tail),
            Err(_) => (None, &rest[..]),
        },
        None => (None, &rest[..]),
    };

    Some(GitVersion {
        major,
        minor,
        patch,
        suffix: (!suffix_parts.is_empty()).then(|| suffix_parts.join(".")),
        raw: raw.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_plain_three_component_version() {
        let v = parse_version("git version 2.45.1\n").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 45, Some(1)));
        assert_eq!(v.suffix, None);
        assert_eq!(v.raw, "2.45.1");
    }

    #[test]
    fn keeps_a_platform_suffix_out_of_the_numbers() {
        let v = parse_version("git version 2.45.1.windows.1\n").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 45, Some(1)));
        assert_eq!(v.suffix.as_deref(), Some("windows.1"));
        assert_eq!(v.raw, "2.45.1.windows.1");
    }

    #[test]
    fn keeps_a_build_tag_in_raw_only() {
        let v = parse_version("git version 2.39.5 (Apple Git-154)\n").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 39, Some(5)));
        assert_eq!(v.suffix, None);
        assert_eq!(v.raw, "2.39.5 (Apple Git-154)");
    }

    #[test]
    fn a_non_numeric_third_component_is_a_suffix_not_a_patch() {
        let v = parse_version("git version 2.46.0.rc1").unwrap();
        assert_eq!(v.patch, Some(0));
        assert_eq!(v.suffix.as_deref(), Some("rc1"));

        let v = parse_version("git version 2.46.rc1").unwrap();
        assert_eq!(v.patch, None);
        assert_eq!(v.suffix.as_deref(), Some("rc1"));
    }

    #[test]
    fn parses_a_two_component_version() {
        let v = parse_version("git version 2.45").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (2, 45, None));
        assert_eq!(v.suffix, None);
    }

    #[test]
    fn finds_the_version_line_in_build_options_output() {
        let out = "git version 2.45.1\ncpu: x86_64\nsizeof-long: 8\n";
        let v = parse_version(out).unwrap();
        assert_eq!((v.major, v.minor), (2, 45));
        assert_eq!(v.raw, "2.45.1");
    }

    #[test]
    fn rejects_output_without_a_version_line() {
        assert!(parse_version("").is_none());
        assert!(parse_version("cpu: x86_64\n").is_none());
        assert!(parse_version("git version\n").is_none());
    }

    #[test]
    fn rejects_non_numeric_leading_components() {
        assert!(parse_version("git version next").is_none());
        assert!(parse_version("git version 2.x.1").is_none());
    }

    #[test]
    fn is_at_least_ignores_the_suffix() {
        let v = parse_version("git version 2.45.1.windows.1").unwrap();
        assert!(v.is_at_least(2, 45, 1));
        assert!(v.is_at_least(1, 99, 99));
        assert!(!v.is_at_least(2, 45, 2));
    }

    #[test]
    fn is_at_least_treats_a_missing_patch_as_zero() {
        let v = parse_version("git version 2.45").unwrap();
        assert!(v.is_at_least(2, 45, 0));
        assert!(!v.is_at_least(2, 45, 1));
    }
}
