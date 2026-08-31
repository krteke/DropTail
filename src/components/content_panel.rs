use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::models::{
    ArchiveFormat, ArchiveSettings, CompressionLevel, ContentItem, ContentItemKind, ContentKind,
    ContentPreview, SendMethod,
};
use crate::presentation::format_size;

#[derive(Debug)]
pub enum ContentPanelMsg {
    Show(ContentPreview),
}

#[derive(Debug)]
pub enum ContentPanelOutput {
    AddFiles,
    AddFolder,
    ChangeSendMethod(SendMethod),
    RemoveItem(String),
}

pub struct ContentPanel {
    preview: ContentPreview,
}

impl ContentPanel {
    fn description(&self) -> String {
        if self.preview.summary.is_ready() {
            format!(
                "{} 个项目 · {}",
                self.preview.summary.item_count,
                format_size(self.preview.summary.total_size_bytes)
            )
        } else {
            "可拖放，也可通过文件选择器添加".to_owned()
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for ContentPanel {
    type Init = ContentPreview;
    type Input = ContentPanelMsg;
    type Output = ContentPanelOutput;

    view! {
        #[root]
        root = gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 16,
            set_margin_top: 28,
            set_margin_bottom: 32,
            set_margin_start: 24,
            set_margin_end: 24,

            adw::WrapBox {
                set_orientation: gtk::Orientation::Horizontal,
                set_child_spacing: 12,
                set_line_spacing: 8,
                set_justify: adw::JustifyMode::Spread,
                set_justify_last_line: true,

                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_width_request: 220,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 2,
                        set_hexpand: true,

                        gtk::Label {
                            set_label: "要发送的内容",
                            set_halign: gtk::Align::Start,
                            add_css_class: "title-3",
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &model.description(),
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.0,
                            set_wrap: true,
                            add_css_class: "dim-label",
                        },
                    },
                },

                append = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Center,

                    #[watch]
                    set_visible: model.preview.summary.is_ready(),

                    gtk::Button {
                        set_label: "添加文件",
                        set_tooltip_text: Some("添加文件"),
                        connect_clicked[sender] => move |_| {
                            sender.output(ContentPanelOutput::AddFiles).ok();
                        },
                    },

                    gtk::Button {
                        set_label: "添加文件夹",
                        set_tooltip_text: Some("添加文件夹"),
                        connect_clicked[sender] => move |_| {
                            sender.output(ContentPanelOutput::AddFolder).ok();
                        },
                    },
                },
            },

            adw::Bin {
                #[watch]
                set_child: Some(&build_content_page(&model.preview, sender.clone())),
            },
        }
    }

    fn init(
        preview: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { preview };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ContentPanelMsg::Show(preview) => self.preview = preview,
        }
    }
}

fn build_content_page(
    preview: &ContentPreview,
    sender: ComponentSender<ContentPanel>,
) -> gtk::Widget {
    match preview.kind {
        ContentKind::Empty => build_empty_page(sender).upcast(),
        ContentKind::Files => build_files_page(preview, sender).upcast(),
        ContentKind::Archive => build_archive_page(preview, sender).upcast(),
    }
}

fn build_empty_page(sender: ComponentSender<ContentPanel>) -> gtk::Frame {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("card");
    frame.set_vexpand(true);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(10)
        .halign(gtk::Align::Fill)
        .valign(gtk::Align::Center)
        .height_request(240)
        .margin_top(28)
        .margin_bottom(28)
        .margin_start(16)
        .margin_end(16)
        .build();

    let icon = gtk::Image::from_icon_name("folder-download-symbolic");
    icon.set_pixel_size(42);
    icon.add_css_class("dim-label");

    let title = gtk::Label::new(Some("拖放文件或文件夹到这里"));
    title.add_css_class("title-3");
    title.set_wrap(true);
    title.set_justify(gtk::Justification::Center);

    let subtitle = gtk::Label::new(Some("也可以使用下面的选择按钮"));
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    subtitle.set_justify(gtk::Justification::Center);

    let actions = adw::WrapBox::builder()
        .orientation(gtk::Orientation::Horizontal)
        .child_spacing(8)
        .line_spacing(8)
        .halign(gtk::Align::Center)
        .justify(adw::JustifyMode::None)
        .justify_last_line(true)
        .build();
    let file_button = gtk::Button::with_label("选择文件…");
    file_button.set_tooltip_text(Some("选择文件"));
    file_button.connect_clicked({
        let sender = sender.clone();
        move |_| {
            sender.output(ContentPanelOutput::AddFiles).ok();
        }
    });
    let folder_button = gtk::Button::with_label("选择文件夹…");
    folder_button.set_tooltip_text(Some("选择文件夹"));
    folder_button.connect_clicked(move |_| {
        sender.output(ContentPanelOutput::AddFolder).ok();
    });
    actions.append(&file_button);
    actions.append(&folder_button);

    content.append(&icon);
    content.append(&title);
    content.append(&subtitle);
    content.append(&actions);
    frame.set_child(Some(&content));
    frame
}

fn build_files_page(preview: &ContentPreview, sender: ComponentSender<ContentPanel>) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(26)
        .build();
    page.append(&build_file_list(&preview.items, sender.clone()));

    let method_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    method_box.append(&section_heading("发送方式"));

    let methods = gtk::ListBox::new();
    methods.set_selection_mode(gtk::SelectionMode::None);
    methods.add_css_class("boxed-list");

    let separate = gtk::CheckButton::new();
    separate.set_active(preview.summary.method == SendMethod::Separate);
    separate.connect_toggled({
        let sender = sender.clone();
        move |button| {
            if button.is_active() {
                sender
                    .output(ContentPanelOutput::ChangeSendMethod(SendMethod::Separate))
                    .ok();
            }
        }
    });
    let separate_row = adw::ActionRow::builder()
        .title("分别发送")
        .activatable(true)
        .build();
    separate_row.add_prefix(&separate);
    separate_row.set_activatable_widget(Some(&separate));
    methods.append(&separate_row);

    let archive = gtk::CheckButton::new();
    archive.set_group(Some(&separate));
    archive.set_active(preview.summary.method == SendMethod::Archive);
    archive.connect_toggled(move |button| {
        if button.is_active() {
            sender
                .output(ContentPanelOutput::ChangeSendMethod(SendMethod::Archive))
                .ok();
        }
    });
    let archive_row = adw::ActionRow::builder()
        .title("打包发送")
        .activatable(true)
        .build();
    archive_row.add_prefix(&archive);
    archive_row.set_activatable_widget(Some(&archive));
    methods.append(&archive_row);

    method_box.append(&methods);
    page.append(&method_box);
    page
}

fn build_archive_page(preview: &ContentPreview, sender: ComponentSender<ContentPanel>) -> gtk::Box {
    let settings = preview
        .archive
        .as_ref()
        .expect("archive preview must include archive settings");
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .build();
    page.append(&build_file_list(&preview.items, sender));

    let settings_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    settings_box.append(&section_heading("发送方式"));
    settings_box.append(&build_archive_settings(settings));

    let note = gtk::Label::new(Some(
        "准备归档时会显示单独进度；临时归档在发送结束或取消后自动清理。",
    ));
    note.set_halign(gtk::Align::Start);
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("caption");
    note.add_css_class("dim-label");
    settings_box.append(&note);

    page.append(&settings_box);
    page
}

fn build_archive_settings(settings: &ArchiveSettings) -> gtk::Box {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let group = adw::PreferencesGroup::new();

    let name_row = adw::ActionRow::builder()
        .title("归档名称")
        .subtitle("接收设备最终看到的文件名")
        .build();
    let name_entry = gtk::Entry::builder()
        .text(&settings.archive_name)
        .hexpand(true)
        .width_chars(28)
        .valign(gtk::Align::Center)
        .build();
    name_row.add_suffix(&name_entry);
    group.add(&name_row);

    let format_row = adw::ComboRow::builder().title("归档格式").build();
    let formats = gtk::StringList::new(&[
        "tar · 仅打包",
        "tar.zst · 体积更小",
        "tar.gz · 通用兼容",
        "zip · 跨平台",
    ]);
    format_row.set_model(Some(&formats));
    format_row.set_selected(match settings.format {
        ArchiveFormat::Auto | ArchiveFormat::TarZst => 0,
        ArchiveFormat::TarGz => 1,
        ArchiveFormat::Zip => 2,
    });
    group.add(&format_row);

    let compression_row = adw::ActionRow::builder().title("压缩级别").build();
    let compression = adw::ToggleGroup::builder()
        .homogeneous(true)
        .can_shrink(true)
        .valign(gtk::Align::Center)
        .build();
    compression.add(adw::Toggle::builder().name("fast").label("快速").build());
    compression.add(
        adw::Toggle::builder()
            .name("balanced")
            .label("平衡")
            .build(),
    );
    compression.add(adw::Toggle::builder().name("smaller").label("更小").build());
    compression.set_active_name(Some(match settings.compression {
        CompressionLevel::Fast => "fast",
        CompressionLevel::Balanced => "balanced",
        CompressionLevel::Smaller => "smaller",
    }));
    compression_row.add_suffix(&compression);
    group.add(&compression_row);
    content.append(&group);

    let advanced = adw::ExpanderRow::builder().title("更多打包选项").build();
    advanced.add_row(&check_row(
        "包含所选文件夹本身",
        Some("关闭后只把文件夹中的内容放入归档根目录。"),
        settings.include_selected_folder,
    ));
    advanced.add_row(&check_row(
        "包含隐藏文件",
        None,
        settings.include_hidden_files,
    ));
    advanced.add_row(&check_row("跟随符号链接", None, settings.follow_symlinks));
    let advanced_group = adw::PreferencesGroup::new();
    advanced_group.add(&advanced);
    content.append(&advanced_group);

    content
}

fn section_heading(title: &str) -> gtk::Box {
    let heading = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .build();
    let title_label = gtk::Label::new(Some(title));
    title_label.set_halign(gtk::Align::Start);
    title_label.add_css_class("title-3");
    heading.append(&title_label);
    heading
}

fn build_file_list(files: &[ContentItem], sender: ComponentSender<ContentPanel>) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");

    for file in files {
        let detail = match file.kind {
            ContentItemKind::File => "文件".to_owned(),
            ContentItemKind::Folder { child_count } => format!("{child_count} 个项目"),
        };
        let row = adw::ActionRow::builder()
            .title(&file.name)
            .subtitle(&detail)
            .build();
        let icon_name = match file.kind {
            ContentItemKind::File => "text-x-generic-symbolic",
            ContentItemKind::Folder { .. } => "folder-symbolic",
        };
        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(20);
        row.add_prefix(&icon);

        let size = gtk::Label::new(Some(&format_size(file.size_bytes)));
        size.add_css_class("dim-label");
        size.set_valign(gtk::Align::Center);
        row.add_suffix(&size);

        let remove = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .tooltip_text("移除此项目")
            .valign(gtk::Align::Center)
            .build();
        remove.add_css_class("flat");
        let item_id = file.id.clone();
        remove.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender
                    .output(ContentPanelOutput::RemoveItem(item_id.clone()))
                    .ok();
            }
        });
        row.add_suffix(&remove);
        list.append(&row);
    }

    list
}

fn check_row(title: &str, subtitle: Option<&str>, active: bool) -> adw::ActionRow {
    let row_builder = adw::ActionRow::builder().title(title);

    let row = if let Some(subtitle) = subtitle {
        row_builder.subtitle(subtitle)
    } else {
        row_builder
    }
    .activatable(true)
    .build();
    let check = gtk::CheckButton::builder()
        .active(active)
        .valign(gtk::Align::Center)
        .build();
    row.add_suffix(&check);
    row.set_activatable_widget(Some(&check));
    row
}
