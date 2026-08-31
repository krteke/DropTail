use crate::models::{ArchiveFormat, CompressionLevel, PreferenceChange, PreferencesData};

pub fn fetch_preferences() -> PreferencesData {
    PreferencesData {
        notify_after_transfer: true,
        inhibit_suspend: true,
        default_format: ArchiveFormat::Auto,
        compression_level: CompressionLevel::Balanced,
    }
}

pub fn update_preference(change: PreferenceChange) {
    match change {
        PreferenceChange::NotifyAfterTransfer(value) | PreferenceChange::InhibitSuspend(value) => {
            _ = value
        }
        PreferenceChange::DefaultFormat(value) => _ = value,
        PreferenceChange::CompressionLevel(value) => _ = value,
    }

    // TODO(integration): send this change to the real settings API.
}
