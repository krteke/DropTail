use crate::domain::content::ContentSelection;
use crate::domain::device::Device;
use crate::domain::transfer::{TransferItemState, TransferQueueItem, TransferSnapshot};

pub fn start_transfer(target: &Device, content: &ContentSelection) -> TransferSnapshot {
    assert!(!content.is_empty());

    let total_bytes = content.total_size_bytes().unwrap_or(0);
    let progress = if total_bytes == 0 { 0.0 } else { 0.17 };
    let transferred_bytes = (total_bytes as f64 * progress) as u64;
    let bytes_per_second = if total_bytes == 0 { 0 } else { 40_055_603 };
    let remaining_bytes = total_bytes.saturating_sub(transferred_bytes);

    let queue = content
        .items()
        .iter()
        .enumerate()
        .map(|(index, item)| TransferQueueItem {
            name: item.name().to_owned(),
            state: if index == 0 {
                TransferItemState::Sending
            } else {
                TransferItemState::Waiting
            },
        })
        .collect::<Vec<_>>();
    let current_name = queue
        .first()
        .expect("ready content must contain at least one item")
        .name
        .clone();

    TransferSnapshot {
        id: "mock-transfer-001".to_owned(),
        target: target.clone(),
        current_name,
        progress,
        transferred_bytes,
        total_bytes,
        bytes_per_second,
        eta_seconds: if bytes_per_second == 0 {
            0
        } else {
            remaining_bytes.div_ceil(bytes_per_second)
        },
        queue,
    }
}

pub fn cancel_transfer(transfer_id: &str) {
    _ = transfer_id;

    // TODO(integration): cancel the transfer through the real Taildrop API.
}
