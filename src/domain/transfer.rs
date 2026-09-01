use crate::domain::device::Device;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferItemState {
    Sending,
    Waiting,
    Sent,
}

#[derive(Debug, Clone)]
pub struct TransferQueueItem {
    pub name: String,
    pub state: TransferItemState,
}

#[derive(Debug, Clone)]
pub struct TransferSnapshot {
    id: u64,
    target: Device,
    current_index: usize,
    transferred_bytes: u64,
    total_bytes: Option<u64>,
    bytes_per_second: u64,
    queue: Vec<TransferQueueItem>,
    cancelling: bool,
}

impl TransferSnapshot {
    pub fn new(id: u64, target: Device, names: Vec<String>, total_bytes: Option<u64>) -> Self {
        assert!(!names.is_empty(), "a transfer must contain files");
        let queue = names
            .into_iter()
            .enumerate()
            .map(|(index, name)| TransferQueueItem {
                name,
                state: if index == 0 {
                    TransferItemState::Sending
                } else {
                    TransferItemState::Waiting
                },
            })
            .collect();

        Self {
            id,
            target,
            current_index: 0,
            transferred_bytes: 0,
            total_bytes,
            bytes_per_second: 0,
            queue,
            cancelling: false,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn item_count(&self) -> usize {
        self.queue.len()
    }

    pub fn current_name(&self) -> &str {
        &self.queue[self.current_index].name
    }

    pub fn target(&self) -> &Device {
        &self.target
    }

    pub fn queue(&self) -> &[TransferQueueItem] {
        &self.queue
    }

    pub fn progress(&self) -> Option<f64> {
        self.total_bytes.map(|total_bytes| {
            if total_bytes == 0 {
                let sent = self
                    .queue
                    .iter()
                    .filter(|item| item.state == TransferItemState::Sent)
                    .count();
                sent as f64 / self.queue.len() as f64
            } else {
                self.transferred_bytes as f64 / total_bytes as f64
            }
        })
    }

    pub fn progress_bytes(&self) -> Option<(u64, u64)> {
        self.total_bytes
            .map(|total_bytes| (self.transferred_bytes, total_bytes))
    }

    pub fn bytes_per_second(&self) -> u64 {
        self.bytes_per_second
    }

    pub fn eta_seconds(&self) -> Option<u64> {
        self.total_bytes.map(|total_bytes| {
            if self.bytes_per_second == 0 {
                0
            } else {
                total_bytes
                    .saturating_sub(self.transferred_bytes)
                    .div_ceil(self.bytes_per_second)
            }
        })
    }

    pub fn is_cancelling(&self) -> bool {
        self.cancelling
    }

    pub fn record_sample(
        &mut self,
        item_index: usize,
        transferred_bytes: u64,
        bytes_per_second: u64,
    ) {
        assert_eq!(
            item_index, self.current_index,
            "progress must belong to the current queue item"
        );
        assert!(
            transferred_bytes >= self.transferred_bytes,
            "transfer progress must be monotonic"
        );
        if let Some(total_bytes) = self.total_bytes {
            assert!(
                transferred_bytes <= total_bytes,
                "transferred bytes cannot exceed the selected content"
            );
        }
        self.transferred_bytes = transferred_bytes;
        self.bytes_per_second = bytes_per_second;
    }

    pub fn finish_item(&mut self, item_index: usize) {
        assert_eq!(
            item_index, self.current_index,
            "only the current queue item can finish"
        );
        self.queue[item_index].state = TransferItemState::Sent;
        if item_index + 1 < self.queue.len() {
            self.current_index += 1;
            self.queue[self.current_index].state = TransferItemState::Sending;
        }
    }

    pub fn request_cancel(&mut self) {
        self.cancelling = true;
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use crate::domain::device::Connection;

    use super::*;

    #[test]
    fn progress_advances_the_queue_and_tracks_the_whole_transfer() {
        let target = Device {
            id: "node-1".to_owned(),
            name: "target".to_owned(),
            platform: "linux".to_owned(),
            address: "100.64.0.2".parse::<IpAddr>().unwrap(),
            connection: Connection::Direct,
        };
        let mut transfer = TransferSnapshot::new(
            7,
            target,
            vec!["first.bin".to_owned(), "second.bin".to_owned()],
            Some(10),
        );

        transfer.record_sample(0, 4, 2);
        assert_eq!(transfer.progress(), Some(0.4));
        assert_eq!(transfer.bytes_per_second(), 2);

        transfer.finish_item(0);
        assert_eq!(transfer.queue[0].state, TransferItemState::Sent);
        assert_eq!(transfer.queue[1].state, TransferItemState::Sending);
        assert_eq!(transfer.current_name(), "second.bin");

        transfer.record_sample(1, 10, 3);
        transfer.finish_item(1);
        assert_eq!(transfer.progress(), Some(1.0));
        assert_eq!(transfer.queue[1].state, TransferItemState::Sent);

        let mut archive = TransferSnapshot::new(
            8,
            transfer.target.clone(),
            vec!["bundle.tar.zst".to_owned()],
            None,
        );
        archive.record_sample(0, 2048, 1024);
        assert_eq!(archive.progress(), None);
        assert_eq!(archive.progress_bytes(), None);
        assert_eq!(archive.eta_seconds(), None);
    }
}
