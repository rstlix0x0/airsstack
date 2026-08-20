//! Validating newtypes for claudevs domain values.

mod case_name;
mod hook_event;
mod ident;
mod marketplace_name;
mod plugin_name;
mod plugin_version;

pub use case_name::{CaseName, InvalidCaseName};
pub use hook_event::{HookEvent, InvalidHookEvent};
pub use marketplace_name::{InvalidMarketplaceName, MarketplaceName};
pub use plugin_name::{InvalidPluginName, PluginName};
pub use plugin_version::{InvalidPluginVersion, PluginVersion};
