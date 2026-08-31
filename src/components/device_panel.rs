use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::models::{Device, DeviceKind, DeviceList};

#[derive(Debug)]
pub enum DevicePanelMsg {
    Select(usize),
}

#[derive(Debug)]
pub enum DevicePanelOutput {
    Selected(Device),
}

pub struct DevicePanel {
    devices: Vec<Device>,
    selected: usize,
}

#[relm4::component(pub)]
impl SimpleComponent for DevicePanel {
    type Init = DeviceList;
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
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let selected = init
            .devices
            .iter()
            .position(|device| device.id == init.selected_id)
            .expect("device panel must receive a valid selected device id");
        let model = Self {
            devices: init.devices,
            selected,
        };
        let widgets = view_output!();

        for device in &model.devices {
            widgets.device_list.append(&build_device_row(device));
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
            DevicePanelMsg::Select(index) => {
                let device = &self.devices[index];
                if device.is_online() {
                    self.selected = index;
                    sender
                        .output(DevicePanelOutput::Selected(device.clone()))
                        .ok();
                }
            }
        }
    }
}

fn build_device_row(device: &Device) -> adw::ActionRow {
    let detail = format!("{} · {}", device.platform, device.address);
    let row = adw::ActionRow::builder()
        .title(&device.name)
        .subtitle(&detail)
        .activatable(device.is_online())
        .sensitive(device.is_online())
        .build();
    row.set_title_lines(1);
    row.set_subtitle_lines(1);

    let icon_name = match device.kind {
        DeviceKind::Computer => "computer-symbolic",
        DeviceKind::Phone => "phone-symbolic",
    };
    let icon = gtk::Image::from_icon_name(icon_name);
    icon.set_pixel_size(24);
    icon.set_margin_start(6);
    icon.set_margin_end(6);
    row.add_prefix(&icon);

    let status = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::End)
        .build();
    let online_label = gtk::Label::new(Some(if device.is_online() {
        "● 在线"
    } else {
        "● 离线"
    }));
    online_label.set_halign(gtk::Align::End);
    online_label.add_css_class("caption");
    status.append(&online_label);
    row.add_suffix(&status);

    row
}
