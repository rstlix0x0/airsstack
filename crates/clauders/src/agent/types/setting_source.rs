//! Which settings sources the binary loads (`--setting-sources`).

/// One source in the binary's `--setting-sources` list: which settings files
/// (and, with them, on-disk agent and command definitions) the binary loads.
///
/// Distinct from [`SettingsSource`](super::SettingsSource), the value of the
/// separate `--settings` flag. The singular/plural split mirrors the binary's
/// own `--setting-sources` vs `--settings`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingSource {
    /// User-level settings (`~/.claude/settings.json`).
    User,
    /// Project-level settings (`.claude/settings.json`).
    Project,
    /// Local project settings (`.claude/settings.local.json`).
    Local,
}

impl SettingSource {
    /// The `--setting-sources` token for this source
    /// (`user` / `project` / `local`), the complete set the binary accepts.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Local => "local",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SettingSource;

    #[test]
    fn as_str_covers_every_source() {
        assert_eq!(SettingSource::User.as_str(), "user");
        assert_eq!(SettingSource::Project.as_str(), "project");
        assert_eq!(SettingSource::Local.as_str(), "local");
    }
}
