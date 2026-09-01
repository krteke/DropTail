use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use thiserror::Error;

use crate::archive::{ArchiveError, ArchiveRequest};
use crate::tailscale::{LocalApiClient, LocalApiError};

pub struct TransferFile {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

#[derive(Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

enum TransferPayload {
    Separate(Vec<TransferFile>),
    Archive(ArchiveRequest),
}

pub struct TransferTask {
    id: u64,
    target_id: String,
    payload: TransferPayload,
    sample_interval: Duration,
    cancellation: CancellationToken,
}

impl TransferTask {
    pub fn separate(
        id: u64,
        target_id: String,
        files: Vec<TransferFile>,
        sample_interval: Duration,
    ) -> Self {
        assert!(!files.is_empty(), "a transfer task must contain files");
        Self::new(
            id,
            target_id,
            TransferPayload::Separate(files),
            sample_interval,
        )
    }

    pub fn archive(
        id: u64,
        target_id: String,
        request: ArchiveRequest,
        sample_interval: Duration,
    ) -> Self {
        Self::new(
            id,
            target_id,
            TransferPayload::Archive(request),
            sample_interval,
        )
    }

    fn new(
        id: u64,
        target_id: String,
        payload: TransferPayload,
        sample_interval: Duration,
    ) -> Self {
        assert!(
            !sample_interval.is_zero(),
            "the speed sample interval must be positive"
        );
        Self {
            id,
            target_id,
            payload,
            sample_interval,
            cancellation: CancellationToken::default(),
        }
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
}

#[derive(Debug)]
pub enum TransferEvent {
    Sample {
        id: u64,
        item_index: usize,
        transferred_bytes: u64,
        bytes_per_second: u64,
    },
    ItemFinished {
        id: u64,
        item_index: usize,
    },
    Finished {
        id: u64,
    },
    Cancelled {
        id: u64,
    },
    Failed {
        id: u64,
        error: TransferError,
    },
}

#[derive(Debug, Error)]
pub enum TransferError {
    #[error("无法读取文件 {}：{source}", .path.display())]
    FileSystem {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("所选路径已不再是常规文件：{}", .0.display())]
    FileExpected(PathBuf),
    #[error(
        "文件 {} 在选择后发生了变化（原为 {selected_size} 字节，现为 {current_size} 字节），请重新添加后再发送",
        .path.display()
    )]
    FileSizeChanged {
        path: PathBuf,
        selected_size: u64,
        current_size: u64,
    },
    #[error(transparent)]
    Archive(#[from] ArchiveError),
    #[error(transparent)]
    LocalApi(#[from] LocalApiError),
}

pub fn run<F>(task: TransferTask, emit: F)
where
    F: Fn(TransferEvent) + Send + Sync + 'static,
{
    let emit = Arc::new(emit);
    let client = match LocalApiClient::new() {
        Ok(client) => client,
        Err(error) => {
            emit(TransferEvent::Failed {
                id: task.id,
                error: error.into(),
            });
            return;
        }
    };

    match task.payload {
        TransferPayload::Separate(files) => run_separate(
            task.id,
            &task.target_id,
            files,
            task.sample_interval,
            task.cancellation,
            &client,
            emit,
        ),
        TransferPayload::Archive(request) => run_archive(
            task.id,
            &task.target_id,
            request,
            task.sample_interval,
            task.cancellation,
            &client,
            emit,
        ),
    }
}

fn run_separate<F>(
    id: u64,
    target_id: &str,
    files: Vec<TransferFile>,
    sample_interval: Duration,
    cancellation: CancellationToken,
    client: &LocalApiClient,
    emit: Arc<F>,
) where
    F: Fn(TransferEvent) + Send + Sync + 'static,
{
    let mut completed_bytes = 0_u64;

    for (item_index, transfer_file) in files.into_iter().enumerate() {
        if cancellation.is_cancelled() {
            emit(TransferEvent::Cancelled { id });
            return;
        }

        let file = match open_selected_file(&transfer_file) {
            Ok(file) => file,
            Err(error) => {
                emit(TransferEvent::Failed { id, error });
                return;
            }
        };
        let sample_emit = Arc::clone(&emit);
        let reader = SampledReader::new(
            file.take(transfer_file.size),
            Some(transfer_file.size),
            sample_interval,
            cancellation.clone(),
            move |file_bytes, bytes_per_second| {
                sample_emit(TransferEvent::Sample {
                    id,
                    item_index,
                    transferred_bytes: completed_bytes + file_bytes,
                    bytes_per_second,
                });
            },
        );

        if let Err(error) = client.push_file(
            target_id,
            &transfer_file.name,
            Some(transfer_file.size),
            reader,
        ) {
            if cancellation.is_cancelled() {
                emit(TransferEvent::Cancelled { id });
            } else {
                emit(TransferEvent::Failed {
                    id,
                    error: error.into(),
                });
            }
            return;
        }

        completed_bytes += transfer_file.size;
        emit(TransferEvent::ItemFinished { id, item_index });
    }

    emit(TransferEvent::Finished { id });
}

fn run_archive<F>(
    id: u64,
    target_id: &str,
    request: ArchiveRequest,
    sample_interval: Duration,
    cancellation: CancellationToken,
    client: &LocalApiClient,
    emit: Arc<F>,
) where
    F: Fn(TransferEvent) + Send + Sync + 'static,
{
    let archive_name = request.name().to_owned();
    let (stream, completion) = match request.stream() {
        Ok(parts) => parts,
        Err(error) => {
            emit(TransferEvent::Failed {
                id,
                error: error.into(),
            });
            return;
        }
    };
    let sample_emit = Arc::clone(&emit);
    let reader = SampledReader::new(
        stream,
        None,
        sample_interval,
        cancellation.clone(),
        move |transferred_bytes, bytes_per_second| {
            sample_emit(TransferEvent::Sample {
                id,
                item_index: 0,
                transferred_bytes,
                bytes_per_second,
            });
        },
    );
    let send_result = client.push_file(target_id, &archive_name, None, reader);
    let archive_result = completion.finish();

    if cancellation.is_cancelled() {
        emit(TransferEvent::Cancelled { id });
        return;
    }

    match (send_result, archive_result) {
        (Ok(()), Ok(())) => {
            emit(TransferEvent::ItemFinished { id, item_index: 0 });
            emit(TransferEvent::Finished { id });
        }
        (_, Err(error)) if !error.is_broken_pipe() => emit(TransferEvent::Failed {
            id,
            error: error.into(),
        }),
        (Err(error), _) => emit(TransferEvent::Failed {
            id,
            error: error.into(),
        }),
        (Ok(()), Err(error)) => emit(TransferEvent::Failed {
            id,
            error: error.into(),
        }),
    }
}

fn open_selected_file(transfer_file: &TransferFile) -> Result<File, TransferError> {
    let file = File::open(&transfer_file.path).map_err(|source| TransferError::FileSystem {
        path: transfer_file.path.clone(),
        source,
    })?;
    let metadata = file
        .metadata()
        .map_err(|source| TransferError::FileSystem {
            path: transfer_file.path.clone(),
            source,
        })?;
    if !metadata.is_file() {
        return Err(TransferError::FileExpected(transfer_file.path.clone()));
    }
    if metadata.len() != transfer_file.size {
        return Err(TransferError::FileSizeChanged {
            path: transfer_file.path.clone(),
            selected_size: transfer_file.size,
            current_size: metadata.len(),
        });
    }

    Ok(file)
}

struct SampledReader<R, F> {
    inner: R,
    expected_size: Option<u64>,
    transferred: u64,
    last_sample_bytes: u64,
    last_sample_at: Instant,
    sample_interval: Duration,
    finished: bool,
    cancellation: CancellationToken,
    emit: F,
}

impl<R, F> SampledReader<R, F> {
    fn new(
        inner: R,
        expected_size: Option<u64>,
        sample_interval: Duration,
        cancellation: CancellationToken,
        emit: F,
    ) -> Self {
        assert!(
            !sample_interval.is_zero(),
            "sample interval must be positive"
        );
        Self {
            inner,
            expected_size,
            transferred: 0,
            last_sample_bytes: 0,
            last_sample_at: Instant::now(),
            sample_interval,
            finished: false,
            cancellation,
            emit,
        }
    }
}

impl<R, F> Read for SampledReader<R, F>
where
    R: Read,
    F: Fn(u64, u64),
{
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.cancellation.is_cancelled() {
            return Err(io::Error::other("transfer cancelled"));
        }

        let read = self.inner.read(buffer)?;
        self.transferred += read as u64;
        if let Some(expected_size) = self.expected_size {
            assert!(
                self.transferred <= expected_size,
                "a sized request body cannot exceed its declared size"
            );
        }
        let now = Instant::now();
        let sample_due = now.duration_since(self.last_sample_at) >= self.sample_interval;
        let reached_expected = read > 0 && self.expected_size == Some(self.transferred);
        let stream_finished =
            read == 0 && !self.finished && self.transferred != self.last_sample_bytes;
        if sample_due || reached_expected || stream_finished {
            let elapsed = now.duration_since(self.last_sample_at);
            let sampled_bytes = self.transferred - self.last_sample_bytes;
            let bytes_per_second = if elapsed.is_zero() {
                0
            } else {
                let rate =
                    u128::from(sampled_bytes).saturating_mul(1_000_000_000) / elapsed.as_nanos();
                rate.min(u128::from(u64::MAX)) as u64
            };
            (self.emit)(self.transferred, bytes_per_second);
            self.last_sample_bytes = self.transferred;
            self.last_sample_at = now;
        }
        if read == 0 {
            self.finished = true;
        }
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn cancelling_a_reader_stops_the_request_body() {
        let cancellation = CancellationToken::default();
        let mut reader = SampledReader::new(
            Cursor::new([1_u8, 2, 3]),
            Some(3),
            Duration::from_millis(200),
            cancellation.clone(),
            |_, _| {},
        );
        cancellation.cancel();

        let error = reader
            .read(&mut [0_u8; 3])
            .expect_err("a cancelled body must stop producing bytes");

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn a_sized_body_emits_its_final_sample_before_the_interval() {
        let samples = Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured_samples = Arc::clone(&samples);
        let mut reader = SampledReader::new(
            Cursor::new([1_u8, 2, 3]),
            Some(3),
            Duration::from_secs(60),
            CancellationToken::default(),
            move |bytes, rate| {
                captured_samples
                    .lock()
                    .expect("sample mutex poisoned")
                    .push((bytes, rate));
            },
        );
        let mut body = Vec::new();
        reader
            .read_to_end(&mut body)
            .expect("sampled body must be readable");

        assert_eq!(body, [1, 2, 3]);
        let samples = samples.lock().expect("sample mutex poisoned");
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].0, 3);
    }
}
