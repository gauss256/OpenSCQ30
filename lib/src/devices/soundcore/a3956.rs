//! Soundcore Liberty 5 Pro.
//!
//! This device speaks a newer variant of the Soundcore protocol where the state packet is a
//! list of tag/length/value records. Only a handful of tags are understood so far. The main
//! purpose of this module is to surface the unsolicited Easy Chat start/stop event
//! (command 0x3103) so that a client can pause and resume media playback.

use std::collections::HashMap;

use crate::devices::soundcore::common::{
    macros::soundcore_device,
    packet::{
        inbound::TryToPacket,
        outbound::{RequestState, ToPacket},
    },
};

mod modules;
mod packets;
mod state;
mod structures;

soundcore_device!(
    state::A3956State,
    async |packet_io| {
        let state_update_packet: packets::inbound::A3956StateUpdatePacket = packet_io
            .send_with_response(&RequestState.to_packet())
            .await?
            .try_to_packet()?;
        tracing::info!(
            "a3956 state tags: {:?}",
            state_update_packet
                .records
                .0
                .iter()
                .map(|(tag, value)| format!("{tag:02x}:{}", hex_string(value)))
                .collect::<Vec<_>>()
        );
        Ok(state::A3956State::new(state_update_packet))
    },
    async |builder| {
        builder.a3956_device_info();
        builder.a3956_easy_chat_status();
    },
    {
        HashMap::from([(
            RequestState::COMMAND,
            packets::inbound::A3956StateUpdatePacket::default().to_packet(),
        )])
    },
);

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        DeviceModel,
        devices::soundcore::common::{
            device::{SoundcoreDeviceConfig, test_utils::TestSoundcoreDevice},
            packet,
        },
        settings::SettingId,
    };

    fn captured_state_body() -> Vec<u8> {
        const HEX: &str = super::packets::inbound::tests::CAPTURED_STATE_BODY;
        (0..HEX.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&HEX[i..i + 2], 16).unwrap())
            .collect()
    }

    #[tokio::test(start_paused = true)]
    async fn parses_captured_liberty_5_pro_state() {
        let device = TestSoundcoreDevice::new(
            super::device_registry,
            DeviceModel::SoundcoreA3956,
            HashMap::from([(
                packet::Command([1, 1]),
                packet::Inbound::new(packet::Command([1, 1]), captured_state_body()),
            )]),
            SoundcoreDeviceConfig::default(),
        )
        .await;

        device.assert_setting_values([
            (SettingId::SerialNumber, "0000000000000000".into()),
            (SettingId::FirmwareVersionLeft, "05.51".into()),
            (SettingId::FirmwareVersionRight, "05.51".into()),
            (SettingId::BatteryLevelLeft, "93%".into()),
            (SettingId::BatteryLevelRight, "92%".into()),
            (SettingId::EasyChatActive, "Idle".into()),
        ]);
    }
}
