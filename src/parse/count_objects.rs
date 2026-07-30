//! Parser for `git count-objects`.
//!
//! The command has two output shapes. `-v` prints one `<key>: <value>` line per
//! statistic and is what [`parse_count_objects`] reads; the default form prints
//! a single `<n> objects, <size>` summary, which [`parse_count_objects_terse`]
//! reads instead.
//!
//! Sizes are the one wrinkle. Without `--human-readable` git prints them as
//! bare kibibyte counts (`size: 20`); with it, as rendered text
//! (`size: 20.00 KiB`, `size-pack: 0 bytes`). Both are kept: [`ObjectSize`]
//! carries git's text and, for the bare form, the numeric value.

/// A size reported by `git count-objects`.
///
/// [`kib`](Self::kib) is `Some` only for the bare-integer form git prints
/// without `--human-readable`; under `--human-readable` the value is rendered
/// text and only [`raw`](Self::raw) is meaningful.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ObjectSize {
    /// The size in kibibytes, when git printed a bare number.
    pub kib: Option<u64>,
    /// The value as git printed it: `20`, `20.00 KiB`, or `0 bytes`.
    pub raw: String,
}

impl ObjectSize {
    /// Parse one size value, keeping the text either way.
    fn new(raw: &str) -> Self {
        Self {
            kib: raw.parse::<u64>().ok(),
            raw: raw.to_string(),
        }
    }
}

/// Statistics from `git count-objects -v`.
///
/// Loose objects are counted by [`count`](Self::count) and packed ones by
/// [`in_pack`](Self::in_pack); the two are disjoint, so a repository's object
/// total is their sum.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CountObjects {
    /// `count`: loose objects.
    pub count: u64,
    /// `size`: disk space the loose objects occupy.
    pub size: ObjectSize,
    /// `in-pack`: objects stored in packs.
    pub in_pack: u64,
    /// `packs`: pack files present.
    pub packs: u64,
    /// `size-pack`: disk space the packs occupy.
    pub size_pack: ObjectSize,
    /// `prune-packable`: loose objects that are also in a pack, so
    /// `git prune-packed` can drop them.
    pub prune_packable: u64,
    /// `garbage`: files in the object store that are neither loose objects nor
    /// packs.
    pub garbage: u64,
    /// `size-garbage`: disk space those garbage files occupy.
    pub size_garbage: ObjectSize,
}

/// Parse the output of `git count-objects -v`.
///
/// Returns [`None`] unless all eight statistics are present, which is the case
/// for `-v` output alone: the default form carries only two of them (see
/// [`parse_count_objects_terse`]). Unrecognized keys are ignored, so a
/// statistic added by a newer git does not break the parse.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_count_objects;
/// let out = "count: 5\nsize: 20\nin-pack: 8\npacks: 1\nsize-pack: 4\n\
///            prune-packable: 0\ngarbage: 0\nsize-garbage: 0\n";
/// let stats = parse_count_objects(out).unwrap();
/// assert_eq!(stats.count, 5);
/// assert_eq!(stats.size.kib, Some(20));
/// assert_eq!(stats.in_pack, 8);
/// ```
#[must_use]
pub fn parse_count_objects(input: &str) -> Option<CountObjects> {
    let mut count = None;
    let mut size = None;
    let mut in_pack = None;
    let mut packs = None;
    let mut size_pack = None;
    let mut prune_packable = None;
    let mut garbage = None;
    let mut size_garbage = None;

    for line in input.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "count" => count = value.parse::<u64>().ok(),
            "size" => size = Some(ObjectSize::new(value)),
            "in-pack" => in_pack = value.parse::<u64>().ok(),
            "packs" => packs = value.parse::<u64>().ok(),
            "size-pack" => size_pack = Some(ObjectSize::new(value)),
            "prune-packable" => prune_packable = value.parse::<u64>().ok(),
            "garbage" => garbage = value.parse::<u64>().ok(),
            "size-garbage" => size_garbage = Some(ObjectSize::new(value)),
            _ => {}
        }
    }

    Some(CountObjects {
        count: count?,
        size: size?,
        in_pack: in_pack?,
        packs: packs?,
        size_pack: size_pack?,
        prune_packable: prune_packable?,
        garbage: garbage?,
        size_garbage: size_garbage?,
    })
}

/// Parse the default `git count-objects` output: `<n> objects, <size>`.
///
/// Returns the loose-object count and the space they occupy, the same two
/// numbers `-v` reports as `count` and `size`. The size text differs between
/// the two forms (`20 kilobytes` here, `20` under `-v`) but
/// [`ObjectSize::kib`] is the same value.
///
/// Returns [`None`] when the line does not have this shape, which includes
/// being handed `-v` output.
///
/// # Example
/// ```
/// use git_spawn::parse::parse_count_objects_terse;
/// let (objects, size) = parse_count_objects_terse("5 objects, 20 kilobytes\n").unwrap();
/// assert_eq!(objects, 5);
/// assert_eq!(size.kib, Some(20));
/// assert_eq!(size.raw, "20 kilobytes");
/// ```
#[must_use]
pub fn parse_count_objects_terse(input: &str) -> Option<(u64, ObjectSize)> {
    let line = input.lines().find(|l| !l.trim().is_empty())?;
    let (objects, size) = line.trim().split_once(", ")?;
    let objects = objects.strip_suffix(" objects")?.parse::<u64>().ok()?;

    // `20 kilobytes` is the bare form's unit word; `20.00 KiB` is the
    // human-readable one. Only the former carries a kibibyte number.
    let kib = size
        .strip_suffix(" kilobytes")
        .and_then(|n| n.parse::<u64>().ok());
    Some((
        objects,
        ObjectSize {
            kib,
            raw: size.to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERBOSE: &str = "count: 5\nsize: 20\nin-pack: 8\npacks: 1\nsize-pack: 4\n\
                           prune-packable: 2\ngarbage: 1\nsize-garbage: 3\n";

    #[test]
    fn parses_every_verbose_statistic() {
        let stats = parse_count_objects(VERBOSE).unwrap();
        assert_eq!(stats.count, 5);
        assert_eq!(stats.size.kib, Some(20));
        assert_eq!(stats.in_pack, 8);
        assert_eq!(stats.packs, 1);
        assert_eq!(stats.size_pack.kib, Some(4));
        assert_eq!(stats.prune_packable, 2);
        assert_eq!(stats.garbage, 1);
        assert_eq!(stats.size_garbage.kib, Some(3));
    }

    #[test]
    fn keeps_human_readable_sizes_as_text_without_a_number() {
        let out = "count: 5\nsize: 20.00 KiB\nin-pack: 8\npacks: 1\n\
                   size-pack: 1.50 KiB\nprune-packable: 0\ngarbage: 0\n\
                   size-garbage: 0 bytes\n";
        let stats = parse_count_objects(out).unwrap();
        assert_eq!(stats.count, 5);
        assert_eq!(
            stats.size,
            ObjectSize {
                kib: None,
                raw: "20.00 KiB".into()
            }
        );
        assert_eq!(stats.size_pack.kib, None);
        assert_eq!(stats.size_pack.raw, "1.50 KiB");
        assert_eq!(stats.size_garbage.raw, "0 bytes");
    }

    #[test]
    fn ignores_an_unrecognized_statistic() {
        let out = format!("{VERBOSE}size-future: 7\n");
        assert_eq!(parse_count_objects(&out), parse_count_objects(VERBOSE));
    }

    #[test]
    fn rejects_output_missing_a_statistic() {
        let out = "count: 5\nsize: 20\nin-pack: 8\npacks: 1\n";
        assert!(parse_count_objects(out).is_none());
    }

    #[test]
    fn rejects_a_non_numeric_count() {
        let out = VERBOSE.replace("count: 5", "count: many");
        assert!(parse_count_objects(&out).is_none());
    }

    #[test]
    fn rejects_the_terse_form() {
        assert!(parse_count_objects("5 objects, 20 kilobytes\n").is_none());
        assert!(parse_count_objects("").is_none());
    }

    #[test]
    fn parses_the_terse_form() {
        let (objects, size) = parse_count_objects_terse("5 objects, 20 kilobytes\n").unwrap();
        assert_eq!(objects, 5);
        assert_eq!(size.kib, Some(20));
        assert_eq!(size.raw, "20 kilobytes");
    }

    #[test]
    fn parses_a_human_readable_terse_size_as_text() {
        let (objects, size) = parse_count_objects_terse("5 objects, 20.00 KiB\n").unwrap();
        assert_eq!(objects, 5);
        assert_eq!(size.kib, None);
        assert_eq!(size.raw, "20.00 KiB");
    }

    #[test]
    fn parses_a_single_object() {
        let (objects, size) = parse_count_objects_terse("1 objects, 4 kilobytes").unwrap();
        assert_eq!(objects, 1);
        assert_eq!(size.kib, Some(4));
    }

    #[test]
    fn rejects_terse_input_of_another_shape() {
        assert!(parse_count_objects_terse("").is_none());
        assert!(parse_count_objects_terse("count: 5\nsize: 20\n").is_none());
        assert!(parse_count_objects_terse("many objects, 20 kilobytes").is_none());
        assert!(parse_count_objects_terse("5 loose, 20 kilobytes").is_none());
    }
}
