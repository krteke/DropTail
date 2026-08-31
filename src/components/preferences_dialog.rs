use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

#[derive(Debug)]
pub enum PreferencesDialogMsg {
    NotifyAfterTransfer(bool),
    InhibitSuspend(bool),
    DefaultFormat(u32),
    CompressionLevel(u32),
}

pub struct PreferencesDialog;

#[relm4::component(pub)]
impl SimpleComponent for PreferencesDialog {
    type Init = ();
    type Input = PreferencesDialogMsg;
    type Output = ();

    view! {
        #[root]
        dialog = adw::PreferencesDialog {
            set_title: "首选项",
            set_content_width: 760,
            set_content_height: 570,
            set_search_enabled: false,

            add = &adw::PreferencesPage {
                adw::PreferencesGroup {
                    set_title: "传输",

                    adw::SwitchRow {
                        set_title: "传输完成后通知",
                        set_subtitle: "窗口在后台或被其他窗口遮住时尤其有用。",
                        set_active: true,
                        connect_active_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::NotifyAfterTransfer(row.is_active()));
                        },
                    },

                    adw::SwitchRow {
                        set_title: "传输时阻止系统挂起",
                        set_subtitle: "仅在正在准备归档或发送文件时请求 inhibit。",
                        set_active: true,
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
                        set_subtitle: "“自动”根据目标系统给出兼容性更好的建议，但每次发送仍可改。",
                        connect_selected_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::DefaultFormat(row.selected()));
                        },
                    },

                    #[name = "compression_row"]
                    adw::ComboRow {
                        set_title: "默认压缩级别",
                        set_subtitle: "只影响启用了压缩的格式。",
                        connect_selected_notify[sender] => move |row| {
                            sender.input(PreferencesDialogMsg::CompressionLevel(row.selected()));
                        },
                    },
                },

                adw::PreferencesGroup {
                    set_description: Some(
                        "刻意没有：带宽限制、强制 DERP/直连、接收目录、接收确认、传输历史、临时归档目录、自动删除源文件。前者属于 Tailscale，后者属于高级压缩工具，而不是易用发送器的核心。"
                    ),
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self;
        let widgets = view_output!();

        let formats = gtk::StringList::new(&["自动", "tar.zst", "tar.gz", "zip"]);
        widgets.format_row.set_model(Some(&formats));
        widgets.format_row.set_selected(0);
        let compression = gtk::StringList::new(&["快速", "平衡", "更小"]);
        widgets.compression_row.set_model(Some(&compression));
        widgets.compression_row.set_selected(1);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PreferencesDialogMsg::NotifyAfterTransfer(value)
            | PreferencesDialogMsg::InhibitSuspend(value) => _ = value,
            PreferencesDialogMsg::DefaultFormat(value)
            | PreferencesDialogMsg::CompressionLevel(value) => _ = value,
        }

        // TODO(integration): persist these UI values when an application settings backend exists.
    }
}
