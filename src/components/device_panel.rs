use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceSummary {
    pub name: &'static str,
    pub platform: &'static str,
    pub address: &'static str,
    pub connection: &'static str,
    pub icon_name: &'static str,
}

impl DeviceSummary {
    pub const PIXEL_9: Self = Self {
        name: "Pixel 9",
        platform: "Android",
        address: "100.96.33.8",
        connection: "DERP · fra",
        icon_name: "phone-symbolic",
    };

    pub fn compose_detail(self) -> String {
        format!("{} · {}", self.platform, self.address)
    }

    pub fn progress_detail(self) -> String {
        format!("{} · {}", self.address, self.connection)
    }
}

const DEVICES: [DeviceSummary; 4] = [
    DeviceSummary {
        name: "ThinkPad X1",
        platform: "Linux",
        address: "100.82.14.27",
        connection: "直连",
        icon_name: "computer-symbolic",
    },
    DeviceSummary::PIXEL_9,
    DeviceSummary {
        name: "Studio PC",
        platform: "Windows",
        address: "100.121.5.19",
        connection: "直连",
        icon_name: "computer-symbolic",
    },
    DeviceSummary {
        name: "旧笔记本",
        platform: "Linux",
        address: "100.77.4.50",
        connection: "离线",
        icon_name: "computer-symbolic",
    },
];

#[derive(Debug)]
pub enum DevicePanelMsg {
    Select(usize),
}

#[derive(Debug)]
pub enum DevicePanelOutput {
    Selected(DeviceSummary),
}

pub struct DevicePanel {
    selected: usize,
}

#[relm4::component(pub)]
impl SimpleComponent for DevicePanel {
    type Init = ();
    type Input = DevicePanelMsg;
    type Output = DevicePanelOutput;

    view! {
        #[root]
        root = gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_margin_top: 24,
            set_margin_start: 24,
            set_margin_end: 24,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 2,

                gtk::Label {
                    set_label: "发送到",
                    set_halign: gtk::Align::Start,
                    add_css_class: "title-3",
                },

                gtk::Label {
                    set_label: "只显示支持 Taildrop 的设备；离线设备保留位置但不可选",
                    set_halign: gtk::Align::Start,
                    set_wrap: true,
                    set_xalign: 0.0,
                    add_css_class: "dim-label",
                },
            },

            #[name = "device_list"]
            gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::Single,
                set_activate_on_single_click: true,
                add_css_class: "boxed-list",

                connect_row_selected[sender] => move |_, row| {
                    if let Some(row) = row {
                        sender.input(DevicePanelMsg::Select(row.index() as usize));
                    }
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { selected: 1 };
        let widgets = view_output!();

        for (index, device) in DEVICES.iter().copied().enumerate() {
            widgets
                .device_list
                .append(&build_device_row(device, index == DEVICES.len() - 1));
        }
        widgets.device_list.select_row(
            widgets
                .device_list
                .row_at_index(model.selected as i32)
                .as_ref(),
        );

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DevicePanelMsg::Select(index) if index < DEVICES.len() - 1 => {
                self.selected = index;
                sender
                    .output(DevicePanelOutput::Selected(DEVICES[index]))
                    .ok();
            }
            DevicePanelMsg::Select(_) => {}
        }
    }
}

fn build_device_row(device: DeviceSummary, offline: bool) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(device.name)
        .subtitle(device.compose_detail())
        .activatable(!offline)
        .sensitive(!offline)
        .build();
    row.set_title_lines(1);
    row.set_subtitle_lines(1);

    let icon = gtk::Image::from_icon_name(device.icon_name);
    icon.set_pixel_size(24);
    icon.set_margin_start(6);
    icon.set_margin_end(6);
    row.add_prefix(&icon);

    let status = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    let online_label = gtk::Label::new(Some(if offline { "● 离线" } else { "● 在线" }));
    online_label.set_halign(gtk::Align::End);
    online_label.add_css_class("caption");
    let connection_label = gtk::Label::new(Some(device.connection));
    connection_label.set_halign(gtk::Align::End);
    connection_label.add_css_class("caption");
    connection_label.add_css_class("dim-label");
    status.append(&online_label);
    status.append(&connection_label);
    row.add_suffix(&status);

    row
}
