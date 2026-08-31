use crate::models::{
    ArchiveFormat, ArchiveSettings, CompressionLevel, ContentItem, ContentItemKind, ContentKind,
    ContentPreview, ContentSummary, SendMethod,
};

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const FILES_TOTAL_BYTES: u64 = 1_964_947_538;
const FOLDER_TOTAL_BYTES: u64 = 1_428_076_626;

pub fn fetch_empty_preview() -> ContentPreview {
    ContentPreview {
        kind: ContentKind::Empty,
        summary: ContentSummary {
            item_count: 0,
            total_size_bytes: 0,
            method: SendMethod::Separate,
        },
        items: Vec::new(),
        archive: None,
    }
}

pub fn fetch_files_preview() -> ContentPreview {
    ContentPreview {
        kind: ContentKind::Files,
        summary: ContentSummary {
            item_count: 3,
            total_size_bytes: FILES_TOTAL_BYTES,
            method: SendMethod::Separate,
        },
        items: selected_files(),
        archive: None,
    }
}

pub fn fetch_packed_files_preview() -> ContentPreview {
    ContentPreview {
        kind: ContentKind::Archive,
        summary: ContentSummary {
            item_count: 3,
            total_size_bytes: FILES_TOTAL_BYTES,
            method: SendMethod::Archive,
        },
        items: selected_files(),
        archive: Some(archive_settings("Taildrop files.tar.zst")),
    }
}

pub fn fetch_folder_preview() -> ContentPreview {
    ContentPreview {
        kind: ContentKind::Archive,
        summary: ContentSummary {
            item_count: 2,
            total_size_bytes: FOLDER_TOTAL_BYTES,
            method: SendMethod::Archive,
        },
        items: vec![
            ContentItem {
                id: "project-assets".to_owned(),
                name: "Project Assets/".to_owned(),
                size_bytes: FOLDER_TOTAL_BYTES,
                kind: ContentItemKind::Folder { child_count: 412 },
            },
            ContentItem {
                id: "readme".to_owned(),
                name: "README.md".to_owned(),
                size_bytes: 18_022,
                kind: ContentItemKind::File,
            },
        ],
        archive: Some(archive_settings("Project Assets + 1 item.tar.zst")),
    }
}

pub fn remove_item(item_id: &str) -> ContentPreview {
    _ = item_id;

    // TODO(integration): remove only the requested item through the real selection API.
    fetch_empty_preview()
}

fn selected_files() -> Vec<ContentItem> {
    vec![
        ContentItem {
            id: "presentation".to_owned(),
            name: "presentation.pdf".to_owned(),
            size_bytes: 18_350_080,
            kind: ContentItemKind::File,
        },
        ContentItem {
            id: "dataset".to_owned(),
            name: "dataset.csv.zst".to_owned(),
            size_bytes: 652 * MIB,
            kind: ContentItemKind::File,
        },
        ContentItem {
            id: "recording".to_owned(),
            name: "recording.mkv".to_owned(),
            size_bytes: 1_256_277_934,
            kind: ContentItemKind::File,
        },
    ]
}

fn archive_settings(archive_name: &str) -> ArchiveSettings {
    ArchiveSettings {
        archive_name: archive_name.to_owned(),
        format: ArchiveFormat::TarZst,
        compression: CompressionLevel::Balanced,
        include_selected_folder: true,
        include_hidden_files: true,
        follow_symlinks: false,
    }
}
