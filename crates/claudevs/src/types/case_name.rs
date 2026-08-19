//! Case identity: the name a verdict is reported under.
//!
//! Responsibilities: [`CaseName`] and [`InvalidCaseName`]. Parsed at
//! construction; downstream code treats possession as proof of validity.

/// A validated case name: non-empty, `[A-Za-z0-9._-]` only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaseName(String);

/// Why a string is not a usable case name.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid case name `{0}`: names are non-empty [A-Za-z0-9._-]")]
pub struct InvalidCaseName(String);

impl CaseName {
    /// Parses a case name.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCaseName`] when `raw` is empty or holds a character
    /// outside `[A-Za-z0-9._-]`.
    pub fn new(raw: impl Into<String>) -> Result<Self, InvalidCaseName> {
        let raw = raw.into();
        let valid = !raw.is_empty()
            && raw
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
        if valid {
            Ok(Self(raw))
        } else {
            Err(InvalidCaseName(raw))
        }
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::fmt::Display for CaseName {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::CaseName;

    #[test]
    fn accepts_the_filename_shaped_names_cases_use() {
        for ok in ["blocks-lockfile", "fires_on_rust_file", "a.b-c_d"] {
            assert!(CaseName::new(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn rejects_empty_and_path_like_names() {
        for bad in ["", "a/b", "a b", "café"] {
            assert!(CaseName::new(bad).is_err(), "{bad}");
        }
    }
}
