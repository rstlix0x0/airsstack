//! The receipt returned by an interrupt.

use serde::{Deserialize, Serialize};

/// Result of an interrupt. Present only when the binary advertises
/// `interrupt_receipt_v1`; older CLIs answer with no `still_queued` field.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptReceipt {
    /// Ids of messages still queued after the interrupt.
    #[serde(default)]
    pub still_queued: Vec<String>,
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "tests assert known-valid fixtures; a panic is the intended failure signal"
    )]
    use super::InterruptReceipt;

    #[test]
    fn interrupt_receipt_binds_still_queued() {
        // [binary v2.1.216]: { still_queued: [...] } filtered to strings.
        let r: InterruptReceipt =
            serde_json::from_str(r#"{"still_queued":["m1","m2"]}"#).expect("deserialize");
        assert_eq!(r.still_queued, vec!["m1", "m2"]);
    }

    #[test]
    fn interrupt_receipt_defaults_when_still_queued_absent() {
        let r: InterruptReceipt = serde_json::from_str("{}").expect("deserialize");
        assert!(r.still_queued.is_empty());
    }
}
