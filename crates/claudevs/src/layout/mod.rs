//! Simulating the shape a plugin has once installed (spec §7).

mod installed;
mod manifest;

pub use installed::Installed;
pub use manifest::{PluginManifest, marketplace_name, read as read_manifest};
