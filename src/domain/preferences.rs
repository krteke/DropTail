use std::time::Duration;

use crate::domain::content::{ArchiveFormat, CompressionLevel};

#[derive(Debug, Clone)]
pub struct Preferences {
    pub notify_after_transfer: bool,
    pub inhibit_suspend: bool,
    pub speed_sample_interval: Duration,
    pub show_offline_devices: bool,
    pub default_format: ArchiveFormat,
    pub compression_level: CompressionLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum PreferenceChange {
    NotifyAfterTransfer(bool),
    InhibitSuspend(bool),
    SpeedSampleInterval(Duration),
    ShowOfflineDevices(bool),
    DefaultFormat(ArchiveFormat),
    CompressionLevel(CompressionLevel),
}
