/// Whether the earbuds are currently in an Easy Chat session (voice detected,
/// audio ducked, transparency on). Reported by the device via the 0x3103 event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EasyChatStatus {
    pub is_active: bool,
}

/// Read-only information parsed from the A3956 TLV state packet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct A3956Info {
    pub serial_number: String,
    pub firmware_left: String,
    pub firmware_right: String,
    pub battery_left: u8,
    pub battery_right: u8,
}

/// The A3956 state packet is a sequence of tag/length/value records rather than the
/// fixed layout used by older Soundcore devices. Unknown tags are kept so they can be
/// inspected for reverse engineering.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TlvRecords(pub Vec<(u8, Vec<u8>)>);

impl TlvRecords {
    pub fn get(&self, tag: u8) -> Option<&[u8]> {
        self.0
            .iter()
            .find(|(t, _)| *t == tag)
            .map(|(_, v)| v.as_slice())
    }
}
