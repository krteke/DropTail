use crate::domain::device::{ConnectionKind, Device, DeviceKind, DeviceList};

pub fn fetch_devices() -> DeviceList {
    DeviceList::new(
        vec![
            Device {
                id: "thinkpad-x1".to_owned(),
                name: "ThinkPad X1".to_owned(),
                kind: DeviceKind::Computer,
                platform: "Linux".to_owned(),
                address: "100.82.14.27".to_owned(),
                connection: ConnectionKind::Online,
            },
            Device {
                id: "pixel-9".to_owned(),
                name: "Pixel 9".to_owned(),
                kind: DeviceKind::Phone,
                platform: "Android".to_owned(),
                address: "100.96.33.8".to_owned(),
                connection: ConnectionKind::Online,
            },
            Device {
                id: "studio-pc".to_owned(),
                name: "Studio PC".to_owned(),
                kind: DeviceKind::Computer,
                platform: "Windows".to_owned(),
                address: "100.121.5.19".to_owned(),
                connection: ConnectionKind::Online,
            },
            Device {
                id: "old-laptop".to_owned(),
                name: "旧笔记本".to_owned(),
                kind: DeviceKind::Computer,
                platform: "Linux".to_owned(),
                address: "100.77.4.50".to_owned(),
                connection: ConnectionKind::Offline,
            },
        ],
        "pixel-9".to_owned(),
    )
}
