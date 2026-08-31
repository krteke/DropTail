use std::path::Path;

use crate::domain::content::{
    ArchiveDefaults, ArchiveFormat, ContentItem, ContentSelection, SendMethod,
};
use crate::domain::device::{Device, DeviceList};
use crate::domain::preferences::{PreferenceChange, Preferences};
use crate::domain::transfer::TransferSnapshot;
use crate::mock_api::{devices as devices_api, transfer as transfer_api};
use crate::settings::{SettingsError, SettingsStore};

pub struct Application {
    settings: SettingsStore,
    devices: DeviceList,
    content: ContentSelection,
    transfer: Option<Box<TransferSnapshot>>,
}

impl Application {
    pub fn new() -> Self {
        Self {
            settings: SettingsStore::new(),
            devices: devices_api::fetch_devices(),
            content: ContentSelection::default(),
            transfer: None,
        }
    }

    pub fn preferences(&self) -> Preferences {
        self.settings.read()
    }

    pub fn visible_devices(&self, show_offline: bool) -> DeviceList {
        self.devices.with_offline_visible(show_offline)
    }

    pub fn selected_device(&self) -> &Device {
        self.devices.selected()
    }

    pub fn select_device(&mut self, selected_id: &str) {
        self.devices.select(selected_id);
    }

    pub fn content(&self) -> &ContentSelection {
        &self.content
    }

    pub fn add_content(&mut self, items: Vec<ContentItem>) {
        let defaults = self.archive_defaults();
        self.content.add(items, defaults);
    }

    pub fn set_send_method(&mut self, method: SendMethod) {
        let defaults = self.archive_defaults();
        self.content.set_send_method(method, defaults);
    }

    pub fn set_archive_format(&mut self, format: ArchiveFormat) {
        self.content.set_archive_format(format);
    }

    pub fn remove_content(&mut self, path: &Path) {
        let defaults = self.archive_defaults();
        self.content.remove(path, defaults);
    }

    pub fn update_preference(&self, change: PreferenceChange) -> Result<(), SettingsError> {
        self.settings.write(change)
    }

    pub fn transfer(&self) -> Option<&TransferSnapshot> {
        self.transfer.as_deref()
    }

    pub fn start_transfer(&mut self) -> &TransferSnapshot {
        assert!(self.transfer.is_none(), "a transfer is already active");
        let snapshot = transfer_api::start_transfer(self.selected_device(), &self.content);
        self.transfer = Some(Box::new(snapshot));
        self.transfer()
            .expect("the transfer was initialized immediately above")
    }

    pub fn cancel_transfer(&mut self) {
        let transfer = self
            .transfer
            .take()
            .expect("an active transfer is required");
        transfer_api::cancel_transfer(&transfer.id);
    }

    fn archive_defaults(&self) -> ArchiveDefaults {
        let preferences = self.settings.read();
        ArchiveDefaults {
            format: preferences.default_format,
            compression: preferences.compression_level,
        }
    }
}
