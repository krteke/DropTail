use crate::domain::device::Device;

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
