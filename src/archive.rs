use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use flate2::write::GzEncoder;
use os_pipe::PipeReader;
use thiserror::Error;
use walkdir::WalkDir;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::domain::content::{ArchiveFormat, ArchiveSettings, ContentItem};

#[derive(Debug, Clone)]
pub struct ArchiveRequest {
    items: Vec<ContentItem>,
    settings: ArchiveSettings,
}

impl ArchiveRequest {
    pub fn new(items: Vec<ContentItem>, settings: ArchiveSettings) -> Self {
        assert!(!items.is_empty(), "an archive must contain selected items");
        Self { items, settings }
    }

    pub fn name(&self) -> &str {
        &self.settings.archive_name
    }

    pub fn stream(self) -> Result<(ArchiveStream, ArchiveCompletion), ArchiveError> {
        let (reader, mut writer) = os_pipe::pipe().map_err(ArchiveError::Pipe)?;
        let outcome = Arc::new(Mutex::new(ProducerOutcome::Running));
        let worker_outcome = Arc::clone(&outcome);
        let worker = thread::Builder::new()
            .name("droptail-archive".to_owned())
            .spawn(move || {
                let result = self.write_archive(&mut writer);
                *worker_outcome
                    .lock()
                    .expect("archive outcome mutex poisoned") = match result {
                    Ok(()) => ProducerOutcome::Finished,
                    Err(error) => ProducerOutcome::Failed(error),
                };
            })
            .map_err(ArchiveError::WorkerStart)?;

        Ok((
            ArchiveStream {
                reader,
                outcome: Arc::clone(&outcome),
            },
            ArchiveCompletion { worker, outcome },
        ))
    }

    fn visit_entries<F>(&self, mut visit: F) -> Result<(), ArchiveError>
    where
        F: FnMut(ArchiveEntry) -> Result<(), ArchiveError>,
    {
        let mut archive_paths = HashSet::new();

        for item in &self.items {
            match item {
                ContentItem::File { path, name, .. } => {
                    let metadata = if self.settings.follow_symlinks {
                        fs::metadata(path)
                    } else {
                        fs::symlink_metadata(path)
                    }
                    .map_err(|source| ArchiveError::FileSystem {
                        path: path.clone(),
                        source,
                    })?;
                    let kind = entry_kind(path, metadata.file_type())?;
                    if !matches!(kind, ArchiveEntryKind::File | ArchiveEntryKind::Symlink) {
                        return Err(ArchiveError::FileExpected(path.clone()));
                    }
                    visit_unique(
                        &mut archive_paths,
                        ArchiveEntry {
                            source: path.clone(),
                            archive_path: PathBuf::from(name),
                            kind,
                        },
                        &mut visit,
                    )?;
                }
                ContentItem::Folder { path, name, .. } => {
                    let metadata =
                        fs::metadata(path).map_err(|source| ArchiveError::FileSystem {
                            path: path.clone(),
                            source,
                        })?;
                    if !metadata.is_dir() {
                        return Err(ArchiveError::FolderExpected(path.clone()));
                    }

                    let include_hidden = self.settings.include_hidden_files;
                    let walker = WalkDir::new(path)
                        .follow_links(self.settings.follow_symlinks)
                        .into_iter()
                        .filter_entry(move |entry| {
                            entry.depth() == 0 || include_hidden || !is_hidden(entry.file_name())
                        });
                    for walked in walker {
                        let walked = walked.map_err(|source| ArchiveError::Walk {
                            path: source
                                .path()
                                .map(Path::to_owned)
                                .unwrap_or_else(|| path.clone()),
                            source,
                        })?;
                        if walked.depth() == 0 && !self.settings.include_selected_folder {
                            continue;
                        }

                        let relative = walked
                            .path()
                            .strip_prefix(path)
                            .expect("walkdir entries must remain below their selected folder root");
                        let archive_path = if self.settings.include_selected_folder {
                            PathBuf::from(name).join(relative)
                        } else {
                            relative.to_owned()
                        };
                        let file_type = if self.settings.follow_symlinks {
                            fs::metadata(walked.path())
                                .map_err(|source| ArchiveError::FileSystem {
                                    path: walked.path().to_owned(),
                                    source,
                                })?
                                .file_type()
                        } else {
                            walked.file_type()
                        };
                        let source = walked.into_path();
                        let kind = entry_kind(&source, file_type)?;
                        visit_unique(
                            &mut archive_paths,
                            ArchiveEntry {
                                source,
                                archive_path,
                                kind,
                            },
                            &mut visit,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl ArchiveRequest {
    fn write_archive<W: Write>(&self, output: &mut W) -> Result<(), ArchiveError> {
        match self.settings.format {
            ArchiveFormat::Tar => self.write_tar(output),
            ArchiveFormat::TarZst => {
                let mut encoder = zstd::stream::write::Encoder::new(
                    output,
                    self.settings.compression.zstd_level(),
                )
                .map_err(ArchiveError::Output)?;
                self.write_tar(&mut encoder)?;
                encoder.finish().map_err(ArchiveError::Output)?;
                Ok(())
            }
            ArchiveFormat::TarGz => {
                let mut encoder = GzEncoder::new(output, self.settings.compression.gzip_level());
                self.write_tar(&mut encoder)?;
                encoder.finish().map_err(ArchiveError::Output)?;
                Ok(())
            }
            ArchiveFormat::Zip => self.write_zip(output),
        }
    }

    fn write_tar<W: Write>(&self, output: W) -> Result<(), ArchiveError> {
        let mut archive = tar::Builder::new(output);
        archive.follow_symlinks(self.settings.follow_symlinks);
        self.visit_entries(|entry| {
            archive
                .append_path_with_name(&entry.source, &entry.archive_path)
                .map_err(|source| ArchiveError::FileSystem {
                    path: entry.source,
                    source,
                })
        })?;
        archive.finish().map_err(ArchiveError::Output)
    }

    fn write_zip<W: Write>(&self, output: W) -> Result<(), ArchiveError> {
        let mut archive = zip::ZipWriter::new_stream(output);
        let base_options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(self.settings.compression.zip_level()));

        self.visit_entries(|entry| match entry.kind {
            ArchiveEntryKind::Directory => archive
                .add_directory_from_path(&entry.archive_path, base_options)
                .map_err(ArchiveError::Zip),
            ArchiveEntryKind::Symlink => {
                let target =
                    fs::read_link(&entry.source).map_err(|source| ArchiveError::FileSystem {
                        path: entry.source.clone(),
                        source,
                    })?;
                archive
                    .add_symlink_from_path(&entry.archive_path, target, base_options)
                    .map_err(ArchiveError::Zip)
            }
            ArchiveEntryKind::File => {
                let mut file =
                    File::open(&entry.source).map_err(|source| ArchiveError::FileSystem {
                        path: entry.source.clone(),
                        source,
                    })?;
                let size = file
                    .metadata()
                    .map_err(|source| ArchiveError::FileSystem {
                        path: entry.source.clone(),
                        source,
                    })?
                    .len();
                archive
                    .start_file_from_path(
                        &entry.archive_path,
                        base_options.large_file(size > u64::from(u32::MAX)),
                    )
                    .map_err(ArchiveError::Zip)?;
                io::copy(&mut file, &mut archive)
                    .map(|_| ())
                    .map_err(|source| ArchiveError::FileSystem {
                        path: entry.source,
                        source,
                    })
            }
        })?;

        archive.finish().map(|_| ()).map_err(ArchiveError::Zip)
    }
}

pub struct ArchiveStream {
    reader: PipeReader,
    outcome: Arc<Mutex<ProducerOutcome>>,
}

pub struct ArchiveCompletion {
    worker: JoinHandle<()>,
    outcome: Arc<Mutex<ProducerOutcome>>,
}

enum ProducerOutcome {
    Running,
    Finished,
    Failed(ArchiveError),
}

#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("无法创建归档流：{0}")]
    Pipe(#[source] io::Error),
    #[error("无法启动归档线程：{0}")]
    WorkerStart(#[source] io::Error),
    #[error("归档线程意外终止")]
    WorkerPanicked,
    #[error("所选文件已不再是常规文件：{}", .0.display())]
    FileExpected(PathBuf),
    #[error("所选文件夹已不再是目录：{}", .0.display())]
    FolderExpected(PathBuf),
    #[error("无法读取归档项目 {}：{source}", .path.display())]
    FileSystem {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("无法遍历归档项目 {}：{source}", .path.display())]
    Walk {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
    #[error("归档中存在重复路径：{}", .0.display())]
    DuplicatePath(PathBuf),
    #[error("归档不支持此文件类型：{}", .0.display())]
    UnsupportedFileType(PathBuf),
    #[error("写入归档失败：{0}")]
    Output(#[source] io::Error),
    #[error("写入 ZIP 归档失败：{0}")]
    Zip(#[source] zip::result::ZipError),
}

impl ArchiveError {
    pub fn is_broken_pipe(&self) -> bool {
        match self {
            Self::FileSystem { source, .. } | Self::Output(source) => {
                source.kind() == io::ErrorKind::BrokenPipe
            }
            Self::Zip(zip::result::ZipError::Io(source)) => {
                source.kind() == io::ErrorKind::BrokenPipe
            }
            _ => false,
        }
    }
}

impl Read for ArchiveStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.reader.read(buffer)?;
        if read != 0 {
            return Ok(read);
        }

        match &*self.outcome.lock().expect("archive outcome mutex poisoned") {
            ProducerOutcome::Finished => Ok(0),
            ProducerOutcome::Failed(error) => Err(io::Error::other(error.to_string())),
            ProducerOutcome::Running => {
                Err(io::Error::other("archive worker stopped unexpectedly"))
            }
        }
    }
}

impl ArchiveCompletion {
    pub fn finish(self) -> Result<(), ArchiveError> {
        let Self { worker, outcome } = self;
        worker.join().map_err(|_| ArchiveError::WorkerPanicked)?;
        let mut outcome = outcome.lock().expect("archive outcome mutex poisoned");
        match std::mem::replace(&mut *outcome, ProducerOutcome::Finished) {
            ProducerOutcome::Finished => Ok(()),
            ProducerOutcome::Failed(error) => Err(error),
            ProducerOutcome::Running => Err(ArchiveError::WorkerPanicked),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ArchiveEntryKind {
    File,
    Directory,
    Symlink,
}

struct ArchiveEntry {
    source: PathBuf,
    archive_path: PathBuf,
    kind: ArchiveEntryKind,
}

fn visit_unique<F>(
    archive_paths: &mut HashSet<PathBuf>,
    entry: ArchiveEntry,
    visit: &mut F,
) -> Result<(), ArchiveError>
where
    F: FnMut(ArchiveEntry) -> Result<(), ArchiveError>,
{
    if !archive_paths.insert(entry.archive_path.clone()) {
        return Err(ArchiveError::DuplicatePath(entry.archive_path));
    }
    visit(entry)
}

fn entry_kind(path: &Path, file_type: fs::FileType) -> Result<ArchiveEntryKind, ArchiveError> {
    if file_type.is_file() {
        Ok(ArchiveEntryKind::File)
    } else if file_type.is_dir() {
        Ok(ArchiveEntryKind::Directory)
    } else if file_type.is_symlink() {
        Ok(ArchiveEntryKind::Symlink)
    } else {
        Err(ArchiveError::UnsupportedFileType(path.to_owned()))
    }
}

fn is_hidden(name: &std::ffi::OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    use flate2::read::GzDecoder;

    use crate::domain::content::CompressionLevel;

    use super::*;

    #[test]
    fn all_formats_stream_an_archive_with_the_same_entries() {
        let directory = test_directory("formats");
        fs::write(directory.join("report.txt"), b"taildrop").expect("test file must be written");
        let expected_path = format!(
            "{}/report.txt",
            directory
                .file_name()
                .expect("test directory must have a name")
                .to_string_lossy()
        );

        for format in [
            ArchiveFormat::Tar,
            ArchiveFormat::TarZst,
            ArchiveFormat::TarGz,
            ArchiveFormat::Zip,
        ] {
            let bytes = streamed_archive(&directory, format);
            match format {
                ArchiveFormat::Tar => assert_tar_entry(&bytes, &expected_path),
                ArchiveFormat::TarZst => {
                    let decoded = zstd::stream::decode_all(Cursor::new(bytes))
                        .expect("zstd archive must decode");
                    assert_tar_entry(&decoded, &expected_path);
                }
                ArchiveFormat::TarGz => {
                    let mut decoded = Vec::new();
                    GzDecoder::new(Cursor::new(bytes))
                        .read_to_end(&mut decoded)
                        .expect("gzip archive must decode");
                    assert_tar_entry(&decoded, &expected_path);
                }
                ArchiveFormat::Zip => {
                    let mut archive =
                        zip::ZipArchive::new(Cursor::new(bytes)).expect("zip archive must open");
                    let mut file = archive
                        .by_name(&expected_path)
                        .expect("zip archive must contain the selected file");
                    let mut contents = String::new();
                    file.read_to_string(&mut contents)
                        .expect("zip entry must be readable");
                    assert_eq!(contents, "taildrop");
                }
            }
        }

        fs::remove_dir_all(directory).expect("test directory must be removed");
    }

    #[test]
    fn a_producer_error_aborts_the_stream_and_remains_typed() {
        let directory = test_directory("missing");
        let missing = directory.join("not-there.txt");
        let request = ArchiveRequest::new(
            vec![ContentItem::file(missing, 1)],
            ArchiveSettings {
                archive_name: "archive.tar".to_owned(),
                format: ArchiveFormat::Tar,
                compression: CompressionLevel::Balanced,
                include_selected_folder: true,
                include_hidden_files: true,
                follow_symlinks: false,
            },
        );
        let (mut stream, completion) = request.stream().expect("archive stream must start");

        stream
            .read_to_end(&mut Vec::new())
            .expect_err("a missing source must abort the request body");
        assert!(matches!(
            completion
                .finish()
                .expect_err("the producer error must be retained"),
            ArchiveError::FileSystem { .. }
        ));
        fs::remove_dir(directory).expect("test directory must be removed");
    }

    #[test]
    fn shared_traversal_applies_root_and_hidden_file_options() {
        let directory = test_directory("options");
        fs::write(directory.join("visible.txt"), b"visible")
            .expect("visible test file must be written");
        fs::write(directory.join(".secret"), b"secret").expect("hidden test file must be written");
        let public = directory.join("public");
        fs::create_dir(&public).expect("public directory must be created");
        fs::write(public.join("nested.txt"), b"nested").expect("nested test file must be written");
        let hidden = directory.join(".hidden");
        fs::create_dir(&hidden).expect("hidden directory must be created");
        fs::write(hidden.join("ignored.txt"), b"ignored")
            .expect("hidden nested file must be written");
        let request = ArchiveRequest::new(
            vec![ContentItem::folder(directory.clone(), 4)],
            ArchiveSettings {
                archive_name: "archive.tar".to_owned(),
                format: ArchiveFormat::Tar,
                compression: CompressionLevel::Balanced,
                include_selected_folder: false,
                include_hidden_files: false,
                follow_symlinks: false,
            },
        );
        let mut paths = Vec::new();

        request
            .visit_entries(|entry| {
                paths.push(entry.archive_path);
                Ok(())
            })
            .expect("archive traversal must succeed");
        paths.sort();

        assert_eq!(
            paths,
            [
                PathBuf::from("public"),
                PathBuf::from("public/nested.txt"),
                PathBuf::from("visible.txt"),
            ]
        );
        fs::remove_dir_all(directory).expect("test directory must be removed");
    }

    fn streamed_archive(directory: &Path, format: ArchiveFormat) -> Vec<u8> {
        let request = ArchiveRequest::new(
            vec![ContentItem::folder(directory.to_owned(), 1)],
            ArchiveSettings {
                archive_name: format!("archive.{}", format.extension()),
                format,
                compression: CompressionLevel::Balanced,
                include_selected_folder: true,
                include_hidden_files: true,
                follow_symlinks: false,
            },
        );
        let (mut stream, completion) = request.stream().expect("archive stream must start");
        let mut bytes = Vec::new();
        stream
            .read_to_end(&mut bytes)
            .expect("archive stream must be readable");
        completion.finish().expect("archive producer must finish");
        bytes
    }

    fn assert_tar_entry(bytes: &[u8], expected_path: &str) {
        let mut archive = tar::Archive::new(Cursor::new(bytes));
        let mut contents = String::new();
        let found = archive
            .entries()
            .expect("tar entries must be readable")
            .find_map(|entry| {
                let mut entry = entry.expect("tar entry must be readable");
                (entry.path().expect("tar path must be valid") == Path::new(expected_path)).then(
                    || {
                        entry
                            .read_to_string(&mut contents)
                            .expect("tar entry contents must be readable");
                    },
                )
            })
            .is_some();
        assert!(found, "tar archive must contain {expected_path}");
        assert_eq!(contents, "taildrop");
    }

    fn test_directory(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("droptail-{name}-{unique}"));
        fs::create_dir(&path).expect("test directory must be created");
        path
    }
}
