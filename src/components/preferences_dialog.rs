use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::models::{ArchiveFormat, CompressionLevel, PreferenceChange, PreferencesData};

#[derive(Debug)]
pub enum PreferencesDialogMsg {
    NotifyAfterTransfer(bool),
    InhibitSuspend(bool),
    ShowOfflineDevices(bool),
    DefaultFormat(u32),
    CompressionLevel(u32),
}

pub struct PreferencesDialog {
    data: PreferencesData,
}

#[relm4::component(pub)]
impl SimpleComponent for PreferencesDialog {
    type Init = PreferencesData;
    type Input = PreferencesDialogMsg;
    type Output = PreferenceChange;

    view! {
        #[root]
        dialog = adw::PreferencesDialog {
            set_title: "首选项",
            set_content_width: 760,
            set_content_height: 570,
            set_search_enabled: false,

            add = &adw::PreferencesPage {
                adw::PreferencesGroup {
                    set_title: "设备",

                    adw::SwitchRow {
                        set_title: "显示离线设备",
                        set_subtitle: "关闭后，设备列表中只保留当前在线的设备。",
                        set_active: model.data.show_offline_devices,
                        connect_active_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::ShowOfflineDevices(row.is_active()));
                        },
                    },
                },

                adw::PreferencesGroup {
                    set_title: "传输",

                    adw::SwitchRow {
                        set_title: "传输完成后通知",
                        set_subtitle: "窗口在后台或被其他窗口遮住时尤其有用。",
                        set_active: model.data.notify_after_transfer,
                        connect_active_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::NotifyAfterTransfer(row.is_active()));
                        },
                    },

                    adw::SwitchRow {
                        set_title: "传输时阻止系统挂起",
                        set_subtitle: "仅在正在准备归档或发送文件时请求 inhibit。",
                        set_active: model.data.inhibit_suspend,
                        connect_active_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::InhibitSuspend(row.is_active()));
                        },
                    },
                },

                adw::PreferencesGroup {
                    set_title: "归档默认值",

                    #[name = "format_row"]
                    adw::ComboRow {
                        set_title: "默认格式",
                        set_subtitle: "默认归档格式。",
                        connect_selected_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::DefaultFormat(row.selected()));
                        },
                    },

                    #[name = "compression_row"]
                    adw::ComboRow {
                        set_title: "默认压缩级别",
                        set_subtitle: "影响启用了压缩的格式。",
                        connect_selected_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::CompressionLevel(row.selected()));
                        },
                    },
                },
            },
        }
    }

    fn init(
        data: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { data };
        let widgets = view_output!();

        let formats = gtk::StringList::new(&["tar", "tar.zst", "tar.gz", "zip"]);
        widgets.format_row.set_model(Some(&formats));
        widgets
            .format_row
            .set_selected(match model.data.default_format {
                ArchiveFormat::Tar => 0,
                ArchiveFormat::TarZst => 1,
                ArchiveFormat::TarGz => 2,
                ArchiveFormat::Zip => 3,
            });

        let compression = gtk::StringList::new(&["快速", "平衡", "更小"]);
        widgets.compression_row.set_model(Some(&compression));
        widgets
            .compression_row
            .set_selected(match model.data.compression_level {
                CompressionLevel::Fast => 0,
                CompressionLevel::Balanced => 1,
                CompressionLevel::Smaller => 2,
            });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        let change = match msg {
            PreferencesDialogMsg::NotifyAfterTransfer(value) => {
                PreferenceChange::NotifyAfterTransfer(value)
            }
            PreferencesDialogMsg::InhibitSuspend(value) => PreferenceChange::InhibitSuspend(value),
            PreferencesDialogMsg::ShowOfflineDevices(value) => {
                PreferenceChange::ShowOfflineDevices(value)
            }
            PreferencesDialogMsg::DefaultFormat(value) => {
                let format = match value {
                    0 => ArchiveFormat::Tar,
                    1 => ArchiveFormat::TarZst,
                    2 => ArchiveFormat::TarGz,
                    3 => ArchiveFormat::Zip,
                    _ => unreachable!("format row only exposes four options"),
                };
                PreferenceChange::DefaultFormat(format)
            }
            PreferencesDialogMsg::CompressionLevel(value) => {
                let level = match value {
                    0 => CompressionLevel::Fast,
                    1 => CompressionLevel::Balanced,
                    2 => CompressionLevel::Smaller,
                    _ => unreachable!("compression row only exposes three options"),
                };
                PreferenceChange::CompressionLevel(level)
            }
        };
        sender.output(change).ok();
    }
}
