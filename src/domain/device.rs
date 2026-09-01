use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Connection {
    Offline,
    Direct,
    Derp(String),
    PeerRelay(String),
}

impl Connection {
    pub fn is_online(&self) -> bool {
        !matches!(self, Self::Offline)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub address: IpAddr,
    pub connection: Connection,
}

impl Device {
    pub fn is_online(&self) -> bool {
        self.connection.is_online()
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeviceList {
    devices: Vec<Device>,
    selected_id: Option<String>,
}

impl DeviceList {
    pub fn new(mut devices: Vec<Device>) -> Self {
        devices.sort_by(|left, right| {
            right
                .is_online()
                .cmp(&left.is_online())
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.id.cmp(&right.id))
        });
        let selected_id = devices
            .iter()
            .find(|device| device.is_online())
            .map(|device| device.id.clone());

        Self {
            devices,
            selected_id,
        }
    }

    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    pub fn selected(&self) -> Option<&Device> {
        let selected_id = self.selected_id.as_deref()?;
        Some(
            self.devices
                .iter()
                .find(|device| device.id == selected_id)
                .expect("selected device must exist"),
        )
    }

    pub fn selected_index(&self) -> Option<usize> {
        let selected_id = self.selected_id.as_deref()?;
        Some(
            self.devices
                .iter()
                .position(|device| device.id == selected_id)
                .expect("selected device must exist"),
        )
    }

    pub fn select(&mut self, selected_id: &str) {
        assert!(
            self.devices
                .iter()
                .any(|device| device.id == selected_id && device.is_online()),
            "selected device must exist and be online"
        );
        self.selected_id = Some(selected_id.to_owned());
    }

    pub fn with_offline_visible(&self, show_offline: bool) -> Self {
        if show_offline {
            return self.clone();
        }

        let devices = self
            .devices
            .iter()
            .filter(|device| device.is_online())
            .cloned()
            .collect();
        Self {
            devices,
            selected_id: self.selected_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_selects_an_online_device_and_can_hide_offline_devices() {
        let offline = device("offline", "A", Connection::Offline);
        let online = device("online", "B", Connection::Direct);
        let list = DeviceList::new(vec![offline, online.clone()]);

        assert_eq!(list.selected(), Some(&online));
        let visible = list.with_offline_visible(false);
        assert_eq!(visible.devices(), &[online]);
        assert_eq!(visible.selected_index(), Some(0));

        let offline_only = DeviceList::new(vec![device("offline", "A", Connection::Offline)]);
        assert_eq!(offline_only.selected(), None);
        assert!(
            offline_only
                .with_offline_visible(false)
                .devices()
                .is_empty()
        );
        assert_eq!(DeviceList::default().selected(), None);
    }

    fn device(id: &str, name: &str, connection: Connection) -> Device {
        Device {
            id: id.to_owned(),
            name: name.to_owned(),
            platform: "Linux".to_owned(),
            address: "100.64.0.1".parse().unwrap(),
            connection,
        }
    }
}
