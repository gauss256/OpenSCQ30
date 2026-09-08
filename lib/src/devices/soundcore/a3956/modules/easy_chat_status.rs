use async_trait::async_trait;
use openscq30_lib_has::Has;
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};
use tokio::sync::watch;

use crate::{
    api::{
        device,
        settings::{CategoryId, Setting, SettingId, Value},
    },
    devices::soundcore::{
        a3956::{packets::inbound::EasyChatEvent, structures::EasyChatStatus},
        common::{
            modules::ModuleCollection,
            packet::{self, inbound::TryToPacket},
            packet_manager::PacketHandler,
            settings_manager::{SettingHandler, SettingHandlerError, SettingHandlerResult},
        },
    },
    i18n::fl,
    macros::enum_subset,
};

enum_subset!(
    SettingId,
    #[derive(EnumString, EnumIter, IntoStaticStr)]
    enum EasyChatStatusSetting {
        EasyChatActive,
    }
);

impl<T> ModuleCollection<T>
where
    T: Has<EasyChatStatus> + Clone + Send + Sync,
{
    pub fn add_a3956_easy_chat_status(&mut self, change_notify: watch::Sender<()>) {
        self.setting_manager
            .add_handler(CategoryId::Miscellaneous, EasyChatStatusSettingHandler);
        self.packet_handlers.set_handler(
            EasyChatEvent::COMMAND,
            Box::new(EasyChatEventPacketHandler { change_notify }),
        );
    }
}

#[derive(Default)]
struct EasyChatStatusSettingHandler;

#[async_trait]
impl<T> SettingHandler<T> for EasyChatStatusSettingHandler
where
    T: Has<EasyChatStatus> + Send,
{
    fn settings(&self) -> Vec<SettingId> {
        EasyChatStatusSetting::iter().map(Into::into).collect()
    }

    fn get(&self, state: &T, setting_id: &SettingId) -> Option<Setting> {
        let status: &EasyChatStatus = state.get();
        let setting: EasyChatStatusSetting = (*setting_id).try_into().ok()?;
        Some(match setting {
            EasyChatStatusSetting::EasyChatActive => Setting::Information {
                value: if status.is_active {
                    "Active".to_owned()
                } else {
                    "Idle".to_owned()
                },
                translated_value: if status.is_active {
                    fl!("easy-chat-active-yes")
                } else {
                    fl!("easy-chat-active-no")
                },
            },
        })
    }

    async fn set(
        &self,
        _state: &mut T,
        _setting_id: &SettingId,
        _value: Value,
    ) -> SettingHandlerResult<()> {
        Err(SettingHandlerError::ReadOnly)
    }
}

/// Holds a `change_notify` sender for two reasons: to notify listeners when an Easy Chat
/// event arrives, and, just as importantly, to keep the sender alive. Without a live sender
/// the device's change watcher exits immediately and no state updates are ever delivered.
struct EasyChatEventPacketHandler {
    change_notify: watch::Sender<()>,
}

#[async_trait]
impl<T> PacketHandler<T> for EasyChatEventPacketHandler
where
    T: Has<EasyChatStatus> + Send + Sync,
{
    async fn handle_packet(
        &self,
        state: &watch::Sender<T>,
        packet: &packet::Inbound,
    ) -> device::Result<()> {
        let event: EasyChatEvent = packet.try_to_packet()?;
        state.send_if_modified(|state| {
            let status: &mut EasyChatStatus = state.get_mut();
            let modified = status.is_active != event.is_active;
            status.is_active = event.is_active;
            modified
        });
        // Wake watch_for_changes directly; unsolicited events must reach the client.
        let _ = self.change_notify.send(());
        tracing::info!("easy chat event: active={}", event.is_active);
        Ok(())
    }
}
