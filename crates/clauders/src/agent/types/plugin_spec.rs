//! A local plugin directory to load (`plugins` startup option).

use std::path::PathBuf;

/// A local plugin directory the binary loads.
///
/// `skip_mcp_discovery` selects `--plugin-dir-no-mcp` over `--plugin-dir`. Only
/// local plugins are modelled: the official SDK supports exactly one plugin
/// type and rejects any other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginSpec {
    /// Filesystem path to the plugin directory.
    pub path: PathBuf,
    /// Load the plugin without discovering its MCP servers.
    pub skip_mcp_discovery: bool,
}

#[cfg(test)]
mod tests {
    use super::PluginSpec;

    #[test]
    fn fields_round_trip_by_value() {
        let a = PluginSpec {
            path: "/plugins/foo".into(),
            skip_mcp_discovery: true,
        };
        assert_eq!(
            a,
            PluginSpec {
                path: "/plugins/foo".into(),
                skip_mcp_discovery: true
            }
        );
        assert_ne!(
            a,
            PluginSpec {
                path: "/plugins/foo".into(),
                skip_mcp_discovery: false
            }
        );
    }
}
