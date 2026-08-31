#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Computer,
    Phone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionKind {
    Online,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub platform: String,
    pub address: String,
    pub connection: ConnectionKind,
}

impl Device {
    pub fn is_online(&self) -> bool {
        self.connection == ConnectionKind::Online
    }
}

#[derive(Debug, Clone)]
pub struct DeviceList {
    devices: Vec<Device>,
    selected_id: String,
}

impl DeviceList {
    pub fn new(devices: Vec<Device>, selected_id: String) -> Self {
        assert!(
            devices.iter().any(|device| device.id == selected_id),
            "selected device must exist"
        );
        Self {
            devices,
            selected_id,
        }
    }

    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    pub fn selected(&self) -> &Device {
        self.devices
            .iter()
            .find(|device| device.id == self.selected_id)
            .expect("selected device must exist")
    }

    pub fn selected_index(&self) -> usize {
        self.devices
            .iter()
            .position(|device| device.id == self.selected_id)
            .expect("selected device must exist")
    }

    pub fn select(&mut self, selected_id: &str) {
        assert!(
            self.devices.iter().any(|device| device.id == selected_id),
            "selected device must exist"
        );
        self.selected_id = selected_id.to_owned();
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
        Self::new(devices, self.selected_id.clone())
    }
}
