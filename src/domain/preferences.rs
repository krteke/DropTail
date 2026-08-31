use crate::domain::content::{ArchiveFormat, CompressionLevel};

#[derive(Debug, Clone)]
pub struct Preferences {
    pub notify_after_transfer: bool,
    pub inhibit_suspend: bool,
    pub show_offline_devices: bool,
    pub default_format: ArchiveFormat,
    pub compression_level: CompressionLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum PreferenceChange {
    NotifyAfterTransfer(bool),
    InhibitSuspend(bool),
    ShowOfflineDevices(bool),
    DefaultFormat(ArchiveFormat),
    CompressionLevel(CompressionLevel),
}
