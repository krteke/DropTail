use crate::mock_api::{content, devices, preferences, transfer};
use crate::models::{
    ContentPreview, Device, DeviceList, PreferenceChange, PreferencesData, SendMethod,
    TransferSnapshot,
};

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

pub struct AppService;

impl AppService {
    pub fn bootstrap(&self) -> AppBootstrap {
        let devices = devices::fetch_devices();
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
            preferences: preferences::fetch_preferences(),
        }
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

    pub fn update_preference(&self, change: PreferenceChange) {
        preferences::update_preference(change);
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
}
