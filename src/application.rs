use relm4::gtk::glib;

use crate::mock_api::{content, devices, transfer};
use crate::models::{
    ContentPreview, Device, DeviceList, PreferenceChange, PreferencesData, SendMethod,
    TransferSnapshot,
};
use crate::settings::SettingsStore;

pub struct AppBootstrap {
    pub devices: DeviceList,
    pub selected_device: Device,
    pub content: ContentPreview,
    pub preferences: PreferencesData,
}

#[derive(Debug, Clone, Copy)]
pub enum ContentRequest {
    Files,
    Folder,
    Method(SendMethod),
}

pub struct AppService {
    settings: SettingsStore,
}

impl AppService {
    pub fn new() -> Self {
        Self {
            settings: SettingsStore::new(),
        }
    }

    pub fn bootstrap(&self) -> AppBootstrap {
        let preferences = self.settings.read();
        let devices = Self::with_device_visibility(
            devices::fetch_devices(),
            preferences.show_offline_devices,
        );
        let selected_device = devices
            .devices
            .iter()
            .find(|device| device.id == devices.selected_id)
            .expect("mock device response must identify an existing device")
            .clone();
        let content = content::fetch_empty_preview();

        AppBootstrap {
            devices,
            selected_device,
            content,
            preferences,
        }
    }

    pub fn fetch_devices(&self, selected_id: &str) -> DeviceList {
        let mut devices = devices::fetch_devices();
        devices.selected_id = selected_id.to_owned();
        Self::with_device_visibility(devices, self.settings.read().show_offline_devices)
    }

    pub fn fetch_content(&self, request: ContentRequest) -> ContentPreview {
        match request {
            ContentRequest::Files | ContentRequest::Method(SendMethod::Separate) => {
                content::fetch_files_preview()
            }
            ContentRequest::Folder => content::fetch_folder_preview(),
            ContentRequest::Method(SendMethod::Archive) => content::fetch_packed_files_preview(),
        }
    }

    pub fn update_preference(&self, change: PreferenceChange) -> Result<(), glib::BoolError> {
        self.settings.write(change)
    }

    pub fn remove_content_item(&self, item_id: &str) -> ContentPreview {
        content::remove_item(item_id)
    }

    pub fn start_transfer(&self, target: &Device, content: &ContentPreview) -> TransferSnapshot {
        transfer::start_transfer(target, content)
    }

    pub fn cancel_transfer(&self, transfer_id: &str) {
        transfer::cancel_transfer(transfer_id);
    }

    fn with_device_visibility(mut devices: DeviceList, show_offline: bool) -> DeviceList {
        if !show_offline {
            devices.devices.retain(Device::is_online);
        }
        assert!(
            devices
                .devices
                .iter()
                .any(|device| device.id == devices.selected_id),
            "selected device must remain visible"
        );
        devices
    }
}
