use std::path::{Path, PathBuf};

use flate2::Compression;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Tar,
    TarZst,
    TarGz,
    Zip,
}

impl ArchiveFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Tar => "tar",
            Self::TarZst => "tar.zst",
            Self::TarGz => "tar.gz",
            Self::Zip => "zip",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    Fast,
    Balanced,
    Smaller,
}

impl CompressionLevel {
    pub fn zstd_level(self) -> i32 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Balanced => 3,
            CompressionLevel::Smaller => 9,
        }
    }

    pub fn gzip_level(self) -> Compression {
        match self {
            CompressionLevel::Fast => Compression::fast(),
            CompressionLevel::Balanced => Compression::default(),
            CompressionLevel::Smaller => Compression::best(),
        }
    }

    pub fn zip_level(self) -> i64 {
        match self {
            CompressionLevel::Fast => 1,
            CompressionLevel::Balanced => 6,
            CompressionLevel::Smaller => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveOption {
    IncludeSelectedFolder,
    IncludeHiddenFiles,
    FollowSymlinks,
}

#[derive(Debug, Clone, Copy)]
pub struct ArchiveDefaults {
    pub format: ArchiveFormat,
    pub compression: CompressionLevel,
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

impl ArchiveSettings {
    fn new(items: &[ContentItem], defaults: ArchiveDefaults) -> Self {
        Self {
            archive_name: archive_name(items, defaults.format),
            format: defaults.format,
            compression: defaults.compression,
            include_selected_folder: true,
            include_hidden_files: true,
            follow_symlinks: false,
        }
    }

    fn refresh_name(&mut self, items: &[ContentItem]) {
        self.archive_name = archive_name(items, self.format);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentItem {
    File {
        path: PathBuf,
        name: String,
        size_bytes: u64,
    },
    Folder {
        path: PathBuf,
        name: String,
        file_count: usize,
    },
}

impl ContentItem {
    pub fn file(path: PathBuf, size_bytes: u64) -> Self {
        let name = display_name(&path);
        Self::File {
            path,
            name,
            size_bytes,
        }
    }

    pub fn folder(path: PathBuf, file_count: usize) -> Self {
        let name = display_name(&path);
        Self::Folder {
            path,
            name,
            file_count,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::File { path, .. } | Self::Folder { path, .. } => path,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::File { name, .. } | Self::Folder { name, .. } => name,
        }
    }

    pub fn size_bytes(&self) -> Option<u64> {
        match self {
            Self::File { size_bytes, .. } => Some(*size_bytes),
            Self::Folder { .. } => None,
        }
    }

    fn is_folder(&self) -> bool {
        matches!(self, Self::Folder { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendMethod {
    Separate,
    Archive,
}

#[derive(Debug, Clone, Default)]
pub enum ContentSelection {
    #[default]
    Empty,
    Separate(Vec<ContentItem>),
    Archive {
        items: Vec<ContentItem>,
        settings: ArchiveSettings,
    },
}

impl ContentSelection {
    pub fn items(&self) -> &[ContentItem] {
        match self {
            Self::Empty => &[],
            Self::Separate(items) | Self::Archive { items, .. } => items,
        }
    }

    pub fn item_count(&self) -> usize {
        self.items().len()
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    pub fn total_size_bytes(&self) -> Option<u64> {
        self.items().iter().try_fold(0_u64, |total, item| {
            item.size_bytes().and_then(|size| total.checked_add(size))
        })
    }

    pub fn send_method(&self) -> SendMethod {
        match self {
            Self::Empty | Self::Separate(_) => SendMethod::Separate,
            Self::Archive { .. } => SendMethod::Archive,
        }
    }

    pub fn can_send_separately(&self) -> bool {
        !self.is_empty() && self.items().iter().all(|item| !item.is_folder())
    }

    pub fn archive_settings(&self) -> Option<&ArchiveSettings> {
        match self {
            Self::Archive { settings, .. } => Some(settings),
            Self::Empty | Self::Separate(_) => None,
        }
    }

    pub fn add(&mut self, additions: Vec<ContentItem>, defaults: ArchiveDefaults) {
        if additions.is_empty() {
            return;
        }

        let mut items = self.items().to_vec();
        for item in additions {
            if let Some(index) = items
                .iter()
                .position(|current| current.path() == item.path())
            {
                items[index] = item;
            } else {
                items.push(item);
            }
        }

        *self = Self::from_parts(
            items,
            self.send_method(),
            self.archive_settings().cloned(),
            defaults,
        );
    }

    pub fn set_send_method(&mut self, method: SendMethod, defaults: ArchiveDefaults) {
        *self = Self::from_parts(
            self.items().to_vec(),
            method,
            self.archive_settings().cloned(),
            defaults,
        );
    }

    pub fn set_archive_format(&mut self, format: ArchiveFormat) {
        let Self::Archive { items, settings } = self else {
            panic!("archive format can only be changed for archived content")
        };
        settings.format = format;
        settings.refresh_name(items);
    }

    pub fn set_archive_name(&mut self, name: String) {
        let Self::Archive { settings, .. } = self else {
            panic!("archive name can only be changed for archived content")
        };
        settings.archive_name = name;
    }

    pub fn set_archive_compression(&mut self, compression: CompressionLevel) {
        let Self::Archive { settings, .. } = self else {
            panic!("compression can only be changed for archived content")
        };
        settings.compression = compression;
    }

    pub fn set_archive_option(&mut self, option: ArchiveOption, active: bool) {
        let Self::Archive { settings, .. } = self else {
            panic!("archive options can only be changed for archived content")
        };
        match option {
            ArchiveOption::IncludeSelectedFolder => settings.include_selected_folder = active,
            ArchiveOption::IncludeHiddenFiles => settings.include_hidden_files = active,
            ArchiveOption::FollowSymlinks => settings.follow_symlinks = active,
        }
    }

    pub fn remove(&mut self, path: &Path, defaults: ArchiveDefaults) {
        let mut items = self.items().to_vec();
        items.retain(|item| item.path() != path);
        *self = Self::from_parts(
            items,
            self.send_method(),
            self.archive_settings().cloned(),
            defaults,
        );
    }

    pub fn move_item(&mut self, from: usize, to: usize) {
        let Self::Separate(items) = self else {
            panic!("only separately sent files can be reordered")
        };
        assert!(from < items.len(), "source item index must exist");
        assert!(to < items.len(), "destination item index must exist");

        if from != to {
            let item = items.remove(from);
            items.insert(to, item);
        }
    }

    fn from_parts(
        items: Vec<ContentItem>,
        requested_method: SendMethod,
        existing_settings: Option<ArchiveSettings>,
        defaults: ArchiveDefaults,
    ) -> Self {
        if items.is_empty() {
            return Self::Empty;
        }

        let requires_archive = items.iter().any(ContentItem::is_folder);
        if requested_method == SendMethod::Separate && !requires_archive {
            return Self::Separate(items);
        }

        let mut settings =
            existing_settings.unwrap_or_else(|| ArchiveSettings::new(&items, defaults));
        settings.refresh_name(&items);
        Self::Archive { items, settings }
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn archive_name(items: &[ContentItem], format: ArchiveFormat) -> String {
    let stem = if let [item] = items {
        match item {
            ContentItem::Folder { name, .. } => name.clone(),
            ContentItem::File { path, name, .. } => path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_else(|| name.clone()),
        }
    } else {
        format!("Taildrop {} items", items.len())
    };

    format!("{stem}.{}", format.extension())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> ArchiveDefaults {
        ArchiveDefaults {
            format: ArchiveFormat::Tar,
            compression: CompressionLevel::Balanced,
        }
    }

    #[test]
    fn selection_preserves_latest_items_and_owns_send_method_invariants() {
        let first = ContentItem::file(PathBuf::from("first.txt"), 5);
        let second = ContentItem::file(PathBuf::from("second.txt"), 6);
        let refreshed_second = ContentItem::file(PathBuf::from("second.txt"), 7);
        let folder = ContentItem::folder(PathBuf::from("assets"), 12);
        let mut selection = ContentSelection::default();

        selection.add(vec![second.clone()], defaults());
        selection.add(vec![first.clone(), refreshed_second], defaults());
        assert_eq!(selection.item_count(), 2);
        assert_eq!(selection.total_size_bytes(), Some(12));
        assert_eq!(selection.send_method(), SendMethod::Separate);
        assert!(selection.can_send_separately());

        selection.set_send_method(SendMethod::Archive, defaults());
        assert_eq!(selection.send_method(), SendMethod::Archive);
        assert!(selection.can_send_separately());

        selection.set_send_method(SendMethod::Separate, defaults());
        assert_eq!(selection.send_method(), SendMethod::Separate);

        selection.add(vec![folder.clone()], defaults());
        assert_eq!(selection.send_method(), SendMethod::Archive);
        assert!(!selection.can_send_separately());
        assert_eq!(selection.total_size_bytes(), None);
        assert!(
            selection
                .archive_settings()
                .expect("folder selection must have archive settings")
                .archive_name
                .ends_with(".tar")
        );

        selection.set_archive_format(ArchiveFormat::TarZst);
        let archive = selection
            .archive_settings()
            .expect("folder selection must have archive settings");
        assert_eq!(archive.format, ArchiveFormat::TarZst);
        assert!(archive.archive_name.ends_with(".tar.zst"));

        selection.set_send_method(SendMethod::Separate, defaults());
        assert_eq!(selection.send_method(), SendMethod::Archive);

        selection.remove(folder.path(), defaults());
        selection.set_send_method(SendMethod::Separate, defaults());
        assert_eq!(selection.send_method(), SendMethod::Separate);
        assert_eq!(selection.total_size_bytes(), Some(12));
    }

    #[test]
    fn separately_sent_files_can_be_moved_in_both_directions() {
        let mut selection = ContentSelection::default();
        selection.add(
            vec![
                ContentItem::file(PathBuf::from("first.txt"), 1),
                ContentItem::file(PathBuf::from("second.txt"), 2),
                ContentItem::file(PathBuf::from("third.txt"), 3),
            ],
            defaults(),
        );

        selection.move_item(0, 2);
        assert_eq!(
            selection
                .items()
                .iter()
                .map(ContentItem::name)
                .collect::<Vec<_>>(),
            ["second.txt", "third.txt", "first.txt"]
        );

        selection.move_item(2, 0);
        assert_eq!(
            selection
                .items()
                .iter()
                .map(ContentItem::name)
                .collect::<Vec<_>>(),
            ["first.txt", "second.txt", "third.txt"]
        );
    }
}
