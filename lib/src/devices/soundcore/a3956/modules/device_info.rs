use async_trait::async_trait;
use openscq30_lib_has::Has;
use strum::{EnumIter, EnumString, IntoEnumIterator, IntoStaticStr};

use crate::{
    api::settings::{CategoryId, Setting, SettingId, Value},
    devices::soundcore::{
        a3956::structures::A3956Info,
        common::{
            modules::ModuleCollection,
            settings_manager::{SettingHandler, SettingHandlerError, SettingHandlerResult},
        },
    },
    macros::enum_subset,
};

enum_subset!(
    SettingId,
    #[derive(EnumString, EnumIter, IntoStaticStr)]
    enum DeviceInfoSetting {
        BatteryLevelLeft,
        BatteryLevelRight,
        FirmwareVersionLeft,
        FirmwareVersionRight,
        SerialNumber,
    }
);

impl<T> ModuleCollection<T>
where
    T: Has<A3956Info> + Clone + Send + Sync,
{
    pub fn add_a3956_device_info(&mut self) {
        self.setting_manager
            .add_handler(CategoryId::DeviceInformation, DeviceInfoSettingHandler);
    }
}

#[derive(Default)]
struct DeviceInfoSettingHandler;

#[async_trait]
impl<T> SettingHandler<T> for DeviceInfoSettingHandler
where
    T: Has<A3956Info> + Send,
{
    fn settings(&self) -> Vec<SettingId> {
        DeviceInfoSetting::iter().map(Into::into).collect()
    }

    fn get(&self, state: &T, setting_id: &SettingId) -> Option<Setting> {
        let info: &A3956Info = state.get();
        let setting: DeviceInfoSetting = (*setting_id).try_into().ok()?;
        fn battery(level: Option<u8>) -> String {
            level.map_or_else(|| "Unknown".to_owned(), |percent| format!("{percent}%"))
        }
        let value = match setting {
            DeviceInfoSetting::BatteryLevelLeft => battery(info.battery_left),
            DeviceInfoSetting::BatteryLevelRight => battery(info.battery_right),
            DeviceInfoSetting::FirmwareVersionLeft => info.firmware_left.clone(),
            DeviceInfoSetting::FirmwareVersionRight => info.firmware_right.clone(),
            DeviceInfoSetting::SerialNumber => info.serial_number.clone(),
        };
        Some(Setting::Information {
            translated_value: value.clone(),
            value,
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
