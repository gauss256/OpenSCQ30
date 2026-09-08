use crate::devices::soundcore::{
    a3956::state::A3956State, common::device::SoundcoreDeviceBuilder,
};

mod device_info;
mod easy_chat_status;

impl SoundcoreDeviceBuilder<A3956State> {
    pub fn a3956_easy_chat_status(&mut self) {
        let change_notify = self.change_notify();
        self.module_collection()
            .add_a3956_easy_chat_status(change_notify);
    }

    pub fn a3956_device_info(&mut self) {
        self.module_collection().add_a3956_device_info();
    }
}
