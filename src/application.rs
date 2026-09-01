use std::path::Path;

use crate::archive::ArchiveRequest;
use crate::domain::content::{
    ArchiveDefaults, ArchiveFormat, ArchiveOption, CompressionLevel, ContentItem, ContentSelection,
    SendMethod,
};
use crate::domain::device::{Device, DeviceList};
use crate::domain::preferences::{PreferenceChange, Preferences};
use crate::domain::transfer::TransferSnapshot;
use crate::settings::{SettingsError, SettingsStore};
use crate::tailscale::{LocalApiClient, LocalApiError};
use crate::transfer::{CancellationToken, TransferFile, TransferTask};

pub fn discover_devices() -> Result<DeviceList, LocalApiError> {
    LocalApiClient::new()?.devices().map(DeviceList::new)
}

struct ActiveTransfer {
    snapshot: TransferSnapshot,
    cancellation: CancellationToken,
}

pub struct Application {
    settings: SettingsStore,
    devices: DeviceList,
    content: ContentSelection,
    transfer: Option<ActiveTransfer>,
    next_transfer_id: u64,
}

impl Application {
    pub fn new() -> Self {
        Self {
            settings: SettingsStore::new(),
            devices: DeviceList::default(),
            content: ContentSelection::default(),
            transfer: None,
            next_transfer_id: 1,
        }
    }

    pub fn preferences(&self) -> Preferences {
        self.settings.read()
    }

    pub fn visible_devices(&self, show_offline: bool) -> DeviceList {
        self.devices.with_offline_visible(show_offline)
    }

    pub fn selected_device(&self) -> Option<&Device> {
        self.devices.selected()
    }

    pub fn replace_devices(&mut self, devices: DeviceList) {
        self.devices = devices;
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

    pub fn set_archive_name(&mut self, name: String) {
        self.content.set_archive_name(name);
    }

    pub fn set_archive_compression(&mut self, compression: CompressionLevel) {
        self.content.set_archive_compression(compression);
    }

    pub fn set_archive_option(&mut self, option: ArchiveOption, active: bool) {
        self.content.set_archive_option(option, active);
    }

    pub fn remove_content(&mut self, path: &Path) {
        let defaults = self.archive_defaults();
        self.content.remove(path, defaults);
    }

    pub fn update_preference(&self, change: PreferenceChange) -> Result<(), SettingsError> {
        self.settings.write(change)
    }

    pub fn transfer(&self) -> Option<&TransferSnapshot> {
        self.transfer.as_ref().map(|transfer| &transfer.snapshot)
    }

    pub fn start_transfer(&mut self) -> TransferTask {
        assert!(self.transfer.is_none(), "a transfer is already active");
        let target = self
            .selected_device()
            .expect("an online target is required to start a transfer")
            .clone();
        let sample_interval = self.settings.read().speed_sample_interval;
        let id = self.next_transfer_id;
        self.next_transfer_id = self
            .next_transfer_id
            .checked_add(1)
            .expect("transfer identifiers must not overflow");
        let (task, names, total_bytes) = match &self.content {
            ContentSelection::Separate(items) => {
                let files = items
                    .iter()
                    .map(|item| TransferFile {
                        path: item.path().to_owned(),
                        name: item.name().to_owned(),
                        size: item
                            .size_bytes()
                            .expect("separate transfers can only contain files"),
                    })
                    .collect::<Vec<_>>();
                let total_bytes = files
                    .iter()
                    .try_fold(0_u64, |total, file| total.checked_add(file.size))
                    .expect("selected file sizes must fit in u64");
                let names = files.iter().map(|file| file.name.clone()).collect();
                (
                    TransferTask::separate(id, target.id.clone(), files, sample_interval),
                    names,
                    Some(total_bytes),
                )
            }
            ContentSelection::Archive { items, settings } => {
                let request = ArchiveRequest::new(items.clone(), settings.clone());
                let names = vec![request.name().to_owned()];
                (
                    TransferTask::archive(id, target.id.clone(), request, sample_interval),
                    names,
                    None,
                )
            }
            ContentSelection::Empty => panic!("selected content is required to start a transfer"),
        };
        self.transfer = Some(ActiveTransfer {
            snapshot: TransferSnapshot::new(id, target, names, total_bytes),
            cancellation: task.cancellation_token(),
        });
        task
    }

    pub fn cancel_transfer(&mut self) {
        let transfer = self
            .transfer
            .as_mut()
            .expect("an active transfer is required");
        transfer.cancellation.cancel();
        transfer.snapshot.request_cancel();
    }

    pub fn record_transfer_sample(
        &mut self,
        id: u64,
        item_index: usize,
        transferred_bytes: u64,
        bytes_per_second: u64,
    ) -> bool {
        let Some(transfer) = self
            .transfer
            .as_mut()
            .filter(|transfer| transfer.snapshot.id() == id)
        else {
            return false;
        };
        transfer
            .snapshot
            .record_sample(item_index, transferred_bytes, bytes_per_second);
        true
    }

    pub fn finish_transfer_item(&mut self, id: u64, item_index: usize) -> bool {
        let Some(transfer) = self
            .transfer
            .as_mut()
            .filter(|transfer| transfer.snapshot.id() == id)
        else {
            return false;
        };
        transfer.snapshot.finish_item(item_index);
        true
    }

    pub fn end_transfer(&mut self, id: u64) -> bool {
        if self.transfer().map(TransferSnapshot::id) != Some(id) {
            return false;
        }
        self.transfer = None;
        true
    }

    fn archive_defaults(&self) -> ArchiveDefaults {
        let preferences = self.settings.read();
        ArchiveDefaults {
            format: preferences.default_format,
            compression: preferences.compression_level,
        }
    }
}
