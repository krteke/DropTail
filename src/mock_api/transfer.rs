use crate::models::{
    ContentPreview, Device, TransferItemState, TransferQueueItem, TransferSnapshot,
};

pub fn start_transfer(target: &Device, content: &ContentPreview) -> TransferSnapshot {
    assert!(content.summary.is_ready());

    let queue = content
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| TransferQueueItem {
            name: item.name.clone(),
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
        progress: 0.17,
        transferred_bytes: 315 * 1024 * 1024,
        total_bytes: content.summary.total_size_bytes,
        bytes_per_second: 40_055_603,
        eta_seconds: 40,
        queue,
    }
}

pub fn cancel_transfer(transfer_id: &str) {
    _ = transfer_id;

    // TODO(integration): cancel the transfer through the real Taildrop API.
}
