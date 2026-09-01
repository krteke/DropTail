use relm4::adw::prelude::*;
use relm4::{Component, ComponentParts, ComponentSender, adw, gtk};

use crate::domain::device::{Device, DeviceList};
use crate::presentation::connection_label;

#[derive(Debug)]
pub enum DevicePanelMsg {
    Show(DeviceList),
    Select(usize),
    Refresh,
    RefreshFinished,
}

#[derive(Debug)]
pub enum DevicePanelOutput {
    Selected(String),
    RefreshRequested,
}

pub struct DevicePanel {
    devices: Vec<Device>,
    refreshing: bool,
}

impl DevicePanel {
    fn replace_devices(&mut self, list: &gtk::ListBox, data: DeviceList) {
        let selected = data.selected_index();

        self.devices = data.devices().to_vec();
        list.remove_all();
        for device in &self.devices {
            list.append(&build_device_row(device));
        }
        if let Some(selected) = selected {
            list.select_row(list.row_at_index(selected as i32).as_ref());
        } else {
            list.unselect_all();
        }
    }
}

#[relm4::component(pub)]
impl Component for DevicePanel {
    type Init = DeviceList;
    type Input = DevicePanelMsg;
    type Output = DevicePanelOutput;
    type CommandOutput = ();

    view! {
        #[root]
        root = gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 12,
            set_margin_top: 24,
            set_margin_start: 24,
            set_margin_end: 24,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 8,

                gtk::Label {
                    set_label: "发送到",
                    set_halign: gtk::Align::Start,
                    set_hexpand: true,
                    add_css_class: "title-3",
                },

                gtk::Button {
                    set_icon_name: "view-refresh-symbolic",
                    set_tooltip_text: Some("刷新设备"),
                    set_valign: gtk::Align::Center,
                    add_css_class: "flat",
                    #[watch]
                    set_sensitive: !model.refreshing,
                    connect_clicked => DevicePanelMsg::Refresh,
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
        let mut model = Self {
            devices: Vec::new(),
            refreshing: false,
        };
        let widgets = view_output!();
        model.replace_devices(&widgets.device_list, init);

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            DevicePanelMsg::Show(devices) => {
                self.replace_devices(&widgets.device_list, devices);
            }
            DevicePanelMsg::Select(index) => {
                let device = &self.devices[index];
                if device.is_online() {
                    sender
                        .output(DevicePanelOutput::Selected(device.id.clone()))
                        .ok();
                }
            }
            DevicePanelMsg::Refresh if !self.refreshing => {
                self.refreshing = true;
                sender.output(DevicePanelOutput::RefreshRequested).ok();
            }
            DevicePanelMsg::Refresh => {}
            DevicePanelMsg::RefreshFinished => self.refreshing = false,
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
    if device.is_online() {
        let path_label = gtk::Label::new(Some(&connection_label(&device.connection)));
        path_label.set_halign(gtk::Align::End);
        path_label.add_css_class("caption");
        path_label.add_css_class("dim-label");
        status.append(&path_label);
    }
    row.add_suffix(&status);

    row
}
