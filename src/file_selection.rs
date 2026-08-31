use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::domain::content::ContentItem;

#[derive(Debug, Error)]
pub enum FileSelectionError {
    #[error("所选路径不是文件：{}", .0.display())]
    FileExpected(PathBuf),
    #[error("所选路径不是文件夹：{}", .0.display())]
    FolderExpected(PathBuf),
    #[error("无法读取 {}：{source}", .path.display())]
    FileSystem {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub fn inspect_files(paths: Vec<PathBuf>) -> Result<Vec<ContentItem>, FileSelectionError> {
    let mut seen = HashSet::with_capacity(paths.len());
    let mut items = Vec::with_capacity(paths.len());

    for path in paths {
        if !seen.insert(path.clone()) {
            continue;
        }

        let metadata = metadata(&path)?;
        if !metadata.is_file() {
            return Err(FileSelectionError::FileExpected(path));
        }
        items.push(ContentItem::file(path, metadata.len()));
    }

    Ok(items)
}

pub fn inspect_folder(path: PathBuf) -> Result<Vec<ContentItem>, FileSelectionError> {
    let metadata = metadata(&path)?;
    if !metadata.is_dir() {
        return Err(FileSelectionError::FolderExpected(path));
    }

    let file_count = folder_file_count(&path)?;
    Ok(vec![ContentItem::folder(path, file_count)])
}

fn metadata(path: &Path) -> Result<fs::Metadata, FileSelectionError> {
    fs::metadata(path).map_err(|source| FileSelectionError::FileSystem {
        path: path.to_owned(),
        source,
    })
}

fn folder_file_count(path: &Path) -> Result<usize, FileSelectionError> {
    let entries = fs::read_dir(path).map_err(|source| FileSelectionError::FileSystem {
        path: path.to_owned(),
        source,
    })?;
    let mut file_count = 0;

    for entry in entries {
        let entry = entry.map_err(|source| FileSelectionError::FileSystem {
            path: path.to_owned(),
            source,
        })?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| FileSelectionError::FileSystem {
                path: entry_path.clone(),
                source,
            })?;
        if file_type.is_file() {
            file_count += 1;
        } else if file_type.is_dir() {
            file_count += folder_file_count(&entry_path)?;
        }
    }

    Ok(file_count)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn test_directory(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("droptail-{name}-{unique}"));
        fs::create_dir(&path).expect("test directory must be created");
        path
    }

    #[test]
    fn files_use_metadata_and_duplicate_paths_are_ignored() {
        let directory = test_directory("files");
        let file = directory.join("report.txt");
        fs::write(&file, b"drop tail").expect("test file must be written");

        let items = inspect_files(vec![file.clone(), file.clone()])
            .expect("selected file must be readable");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].path(), file);
        assert_eq!(items[0].size_bytes(), Some(9));
        fs::remove_dir_all(directory).expect("test directory must be removed");
    }

    #[test]
    fn folder_scan_counts_regular_files_recursively_without_calculating_size() {
        let directory = test_directory("folder");
        fs::write(directory.join("one.bin"), [0_u8; 3]).expect("test file must be written");
        let nested = directory.join("nested");
        fs::create_dir(&nested).expect("nested test directory must be created");
        fs::write(nested.join("two.bin"), [0_u8; 5]).expect("nested test file must be written");

        let items = inspect_folder(directory.clone()).expect("selected folder must be readable");

        assert!(matches!(
            items[0],
            ContentItem::Folder { file_count: 2, .. }
        ));
        assert_eq!(items[0].size_bytes(), None);
        fs::remove_dir_all(directory).expect("test directory must be removed");
    }
}
