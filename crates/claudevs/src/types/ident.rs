//! The identifier predicate the domain newtypes share.
//!
//! Every value in this module names one filesystem path segment (a plugin
//! directory, a marketplace directory, a version directory), so the accepted
//! set is deliberately narrow: rejecting `..` and `/` at construction is what
//! makes a later `join` unable to leave the cache root.

/// Whether `raw` is a non-empty run of ASCII alphanumerics and `extra`.
pub(super) fn is_segment(raw: &str, extra: &str) -> bool {
    !raw.is_empty()
        && raw != "."
        && raw != ".."
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
}

#[cfg(test)]
mod tests {
    use super::is_segment;

    #[test]
    fn rejects_a_traversal_segment_regardless_of_the_extra_set() {
        // `Path::new("/cache/a/p").join("../../../etc")` does not normalize —
        // the `..` components survive in the joined path — so the only bound
        // against a cache-root escape is refusing `..` here, before any join
        // happens.
        assert!(!is_segment("..", "._-"));
        assert!(!is_segment("..", ".+-"));
    }

    #[test]
    fn rejects_a_path_separator() {
        assert!(!is_segment("a/b", "._-"));
    }

    #[test]
    fn accepts_a_plain_alphanumeric_segment() {
        assert!(is_segment("airsstack", "._-"));
    }
}
