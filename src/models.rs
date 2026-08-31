#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Computer,
    Phone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionKind {
    Online,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub platform: String,
    pub address: String,
    pub connection: ConnectionKind,
}

impl Device {
    pub fn is_online(&self) -> bool {
        self.connection == ConnectionKind::Online
    }
}

#[derive(Debug, Clone)]
pub struct DeviceList {
    pub devices: Vec<Device>,
    pub selected_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Empty,
    Files,
    Archive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentItemKind {
    File,
    Folder { child_count: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendMethod {
    Separate,
    Archive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Auto,
    TarZst,
    TarGz,
    Zip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    Fast,
    Balanced,
    Smaller,
}

#[derive(Debug, Clone)]
pub struct ContentSummary {
    pub item_count: usize,
    pub total_size_bytes: u64,
    pub method: SendMethod,
}

impl ContentSummary {
    pub fn is_ready(&self) -> bool {
        self.item_count > 0
    }
}

#[derive(Debug, Clone)]
pub struct ContentItem {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub kind: ContentItemKind,
}

#[derive(Debug, Clone)]
pub struct ArchiveSettings {
    pub archive_name: String,
    pub format: ArchiveFormat,
    pub compression: CompressionLevel,
    pub include_selected_folder: bool,
    pub include_hidden_files: bool,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone)]
pub struct ContentPreview {
    pub kind: ContentKind,
    pub summary: ContentSummary,
    pub items: Vec<ContentItem>,
    pub archive: Option<ArchiveSettings>,
}

#[derive(Debug, Clone)]
pub struct PreferencesData {
    pub notify_after_transfer: bool,
    pub inhibit_suspend: bool,
    pub default_format: ArchiveFormat,
    pub compression_level: CompressionLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum PreferenceChange {
    NotifyAfterTransfer(bool),
    InhibitSuspend(bool),
    DefaultFormat(ArchiveFormat),
    CompressionLevel(CompressionLevel),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferItemState {
    Sending,
    Waiting,
}

#[derive(Debug, Clone)]
pub struct TransferQueueItem {
    pub name: String,
    pub state: TransferItemState,
}

#[derive(Debug, Clone)]
pub struct TransferSnapshot {
    pub id: String,
    pub target: Device,
    pub current_name: String,
    pub progress: f64,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub bytes_per_second: u64,
    pub eta_seconds: u64,
    pub queue: Vec<TransferQueueItem>,
}

impl TransferSnapshot {
    pub fn item_count(&self) -> usize {
        self.queue.len()
    }
}
