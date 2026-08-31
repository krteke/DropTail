use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use super::device_panel::DeviceSummary;

#[derive(Debug)]
pub enum ProgressPanelMsg {
    SetTarget(DeviceSummary),
}

pub struct ProgressPanel {
    target: DeviceSummary,
}

#[relm4::component(pub)]
impl SimpleComponent for ProgressPanel {
    type Init = DeviceSummary;
    type Input = ProgressPanelMsg;
    type Output = ();

    view! {
        #[root]
        root = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 24,
                set_margin_top: 28,
                set_margin_bottom: 32,
                set_margin_start: 24,
                set_margin_end: 24,

                gtk::ListBox {
                    set_selection_mode: gtk::SelectionMode::None,
                    add_css_class: "boxed-list",

                    adw::ActionRow {
                        #[watch]
                        set_title: model.target.name,
                        #[watch]
                        set_subtitle: &model.target.progress_detail(),

                        #[name = "target_icon"]
                        add_prefix = &gtk::Image {
                            #[watch]
                            set_icon_name: Some(model.target.icon_name),
                            set_pixel_size: 24,
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 2,

                        gtk::Label {
                            set_label: "正在发送",
                            set_halign: gtk::Align::Start,
                            add_css_class: "title-3",
                        },

                        gtk::Label {
                            set_label: "当前文件与总体进度",
                            set_halign: gtk::Align::Start,
                            add_css_class: "dim-label",
                        },
                    },

                    gtk::Frame {
                        add_css_class: "card",

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 14,
                            set_margin_top: 20,
                            set_margin_bottom: 20,
                            set_margin_start: 20,
                            set_margin_end: 20,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 2,
                                    set_hexpand: true,

                                    gtk::Label {
                                        set_label: "presentation.pdf",
                                        set_halign: gtk::Align::Start,
                                        add_css_class: "heading",
                                    },

                                    gtk::Label {
                                        set_label: "Taildrop 正在发送",
                                        set_halign: gtk::Align::Start,
                                        add_css_class: "dim-label",
                                    },
                                },

                                gtk::Label {
                                    set_label: "17%",
                                    set_valign: gtk::Align::Start,
                                    add_css_class: "title-3",
                                },
                            },

                            gtk::ProgressBar {
                                set_fraction: 0.17,
                            },

                            adw::WrapBox {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_child_spacing: 12,
                                set_line_spacing: 4,
                                set_justify: adw::JustifyMode::Spread,
                                set_justify_last_line: true,

                                append = &gtk::Label {
                                    set_label: "315 MiB / 1.83 GiB",
                                    set_halign: gtk::Align::Start,
                                    add_css_class: "dim-label",
                                },

                                append = &gtk::Label {
                                    set_label: "38.2 MiB/s   约 40 秒",
                                    set_halign: gtk::Align::End,
                                    add_css_class: "dim-label",
                                },
                            },
                        },
                    },

                    gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                        add_css_class: "boxed-list",

                        adw::ActionRow {
                            set_title: "presentation.pdf",
                            add_suffix = &gtk::Label {
                                set_label: "发送中",
                                add_css_class: "pill",
                            },
                        },

                        adw::ActionRow {
                            set_title: "dataset.csv.zst",
                            add_suffix = &gtk::Label {
                                set_label: "等待",
                                add_css_class: "pill",
                            },
                        },

                        adw::ActionRow {
                            set_title: "recording.mkv",
                            add_suffix = &gtk::Label {
                                set_label: "等待",
                                add_css_class: "pill",
                            },
                        },
                    },
            },
        }
    }

    fn init(
        target: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { target };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ProgressPanelMsg::SetTarget(target) => self.target = target,
        }

        // TODO(integration): replace the static progress fixture with transfer events.
    }
}
