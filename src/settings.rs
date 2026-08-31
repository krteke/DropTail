use relm4::gtk::gio::{self, prelude::*};
use relm4::gtk::glib;
use thiserror::Error;

use crate::domain::content::{ArchiveFormat, CompressionLevel};
use crate::domain::preferences::{PreferenceChange, Preferences};

const SCHEMA_ID: &str = "io.github.krteke.DropTail";

pub struct SettingsStore {
    settings: gio::Settings,
}

#[derive(Debug, Error)]
#[error("无法保存首选项：{0}")]
pub struct SettingsError(#[from] glib::BoolError);

impl SettingsStore {
    pub fn new() -> Self {
        let default_source = gio::SettingsSchemaSource::default();
        let schema = default_source
            .as_ref()
            .and_then(|source| source.lookup(SCHEMA_ID, true))
            .unwrap_or_else(|| {
                let source = gio::SettingsSchemaSource::from_directory(
                    env!("DROPTAIL_GSETTINGS_SCHEMA_DIR"),
                    default_source.as_ref(),
                    false,
                )
                .expect("Cargo-built GSettings schemas must be readable");
                source
                    .lookup(SCHEMA_ID, true)
                    .expect("DropTail GSettings schema must be installed or built")
            });

        Self {
            settings: gio::Settings::new_full(&schema, gio::SettingsBackend::NONE, None),
        }
    }

    pub fn read(&self) -> Preferences {
        Preferences {
            notify_after_transfer: self.settings.boolean("notify-after-transfer"),
            inhibit_suspend: self.settings.boolean("inhibit-suspend"),
            show_offline_devices: self.settings.boolean("show-offline-devices"),
            default_format: match self.settings.enum_("default-format") {
                0 => ArchiveFormat::Tar,
                1 => ArchiveFormat::TarZst,
                2 => ArchiveFormat::TarGz,
                3 => ArchiveFormat::Zip,
                _ => unreachable!("GSettings schema restricts the archive format enum"),
            },
            compression_level: match self.settings.enum_("compression-level") {
                0 => CompressionLevel::Fast,
                1 => CompressionLevel::Balanced,
                2 => CompressionLevel::Smaller,
                _ => unreachable!("GSettings schema restricts the compression level enum"),
            },
        }
    }

    pub fn write(&self, change: PreferenceChange) -> Result<(), SettingsError> {
        match change {
            PreferenceChange::NotifyAfterTransfer(value) => {
                self.settings.set_boolean("notify-after-transfer", value)
            }
            PreferenceChange::InhibitSuspend(value) => {
                self.settings.set_boolean("inhibit-suspend", value)
            }
            PreferenceChange::ShowOfflineDevices(value) => {
                self.settings.set_boolean("show-offline-devices", value)
            }
            PreferenceChange::DefaultFormat(value) => self.settings.set_enum(
                "default-format",
                match value {
                    ArchiveFormat::Tar => 0,
                    ArchiveFormat::TarZst => 1,
                    ArchiveFormat::TarGz => 2,
                    ArchiveFormat::Zip => 3,
                },
            ),
            PreferenceChange::CompressionLevel(value) => self.settings.set_enum(
                "compression-level",
                match value {
                    CompressionLevel::Fast => 0,
                    CompressionLevel::Balanced => 1,
                    CompressionLevel::Smaller => 2,
                },
            ),
        }?;
        Ok(())
    }
}
