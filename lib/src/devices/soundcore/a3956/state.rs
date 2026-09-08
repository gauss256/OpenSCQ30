use openscq30_lib_macros::Has;

use crate::devices::soundcore::a3956::{
    packets::inbound::A3956StateUpdatePacket,
    structures::{A3956Info, EasyChatStatus, TlvRecords},
};

#[derive(Debug, Clone, PartialEq, Eq, Has)]
pub struct A3956State {
    info: A3956Info,
    easy_chat_status: EasyChatStatus,
    raw_state: TlvRecords,
}

impl A3956State {
    pub fn new(packet: A3956StateUpdatePacket) -> Self {
        Self {
            info: packet.info(),
            easy_chat_status: EasyChatStatus::default(),
            raw_state: packet.records,
        }
    }
}
