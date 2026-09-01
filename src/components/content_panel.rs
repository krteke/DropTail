use std::path::PathBuf;

use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::domain::content::{
    ArchiveFormat, ArchiveOption, ArchiveSettings, CompressionLevel, ContentItem, ContentSelection,
    SendMethod,
};
use crate::presentation::format_size;

#[derive(Debug)]
pub enum ContentPanelMsg {
    Show(ContentSelection),
}

#[derive(Debug)]
pub enum ContentPanelOutput {
    AddFiles,
    AddFolder,
    ChangeSendMethod(SendMethod),
    ChangeArchiveFormat(ArchiveFormat),
    ChangeArchiveName(String),
    ChangeArchiveCompression(CompressionLevel),
    ChangeArchiveOption(ArchiveOption, bool),
    RemoveItem(PathBuf),
}

pub struct ContentPanel {
    selection: ContentSelection,
}

impl ContentPanel {
    fn description(&self) -> String {
        if self.selection.is_empty() {
            "拖放或通过文件选择器添加".to_owned()
        } else {
            match self.selection.total_size_bytes() {
                Some(size) => format!(
                    "{} 个项目 · {}",
                    self.selection.item_count(),
                    format_size(size)
                ),
                None => format!("{} 个项目", self.selection.item_count()),
            }
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for ContentPanel {
    type Init = ContentSelection;
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
                    set_visible: !model.selection.is_empty(),

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
                set_child: Some(&build_content_page(&model.selection, sender.clone())),
            },
        }
    }

    fn init(
        selection: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { selection };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ContentPanelMsg::Show(selection) => self.selection = selection,
        }
    }
}

fn build_content_page(
    selection: &ContentSelection,
    sender: ComponentSender<ContentPanel>,
) -> gtk::Widget {
    match selection {
        ContentSelection::Empty => build_empty_page(sender).upcast(),
        ContentSelection::Separate(_) => build_files_page(selection, sender).upcast(),
        ContentSelection::Archive { .. } => build_archive_page(selection, sender).upcast(),
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

    let title = gtk::Label::new(Some("拖放文件或文件夹到此处"));
    title.add_css_class("title-3");
    title.set_wrap(true);
    title.set_justify(gtk::Justification::Center);

    let actions = adw::WrapBox::builder()
        .orientation(gtk::Orientation::Horizontal)
        .child_spacing(8)
        .line_spacing(8)
        .halign(gtk::Align::Center)
        .justify(adw::JustifyMode::None)
        .justify_last_line(true)
        .build();
    let file_button = gtk::Button::with_label("选择文件...");
    file_button.set_tooltip_text(Some("选择文件"));
    file_button.connect_clicked({
        let sender = sender.clone();
        move |_| {
            sender.output(ContentPanelOutput::AddFiles).ok();
        }
    });
    let folder_button = gtk::Button::with_label("选择文件夹...");
    folder_button.set_tooltip_text(Some("选择文件夹"));
    folder_button.connect_clicked(move |_| {
        sender.output(ContentPanelOutput::AddFolder).ok();
    });
    actions.append(&file_button);
    actions.append(&folder_button);

    content.append(&icon);
    content.append(&title);
    content.append(&actions);
    frame.set_child(Some(&content));
    frame
}

fn build_files_page(
    selection: &ContentSelection,
    sender: ComponentSender<ContentPanel>,
) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(26)
        .build();
    page.append(&build_file_list(selection.items(), sender.clone()));

    let method_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    method_box.append(&section_heading("发送方式"));
    method_box.append(&build_send_method_selector(selection, sender));
    page.append(&method_box);
    page
}

fn build_archive_page(
    selection: &ContentSelection,
    sender: ComponentSender<ContentPanel>,
) -> gtk::Box {
    let settings = selection
        .archive_settings()
        .expect("archive preview must include archive settings");
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .build();
    page.append(&build_file_list(selection.items(), sender.clone()));

    let settings_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    settings_box.append(&section_heading("发送方式"));

    if selection.can_send_separately() {
        settings_box.append(&build_send_method_selector(selection, sender.clone()));
    }

    settings_box.append(&build_archive_settings(settings, sender));

    page.append(&settings_box);
    page
}

fn build_send_method_selector(
    selection: &ContentSelection,
    sender: ComponentSender<ContentPanel>,
) -> gtk::ListBox {
    assert!(
        selection.can_send_separately(),
        "send method selection requires file-only content"
    );

    let methods = gtk::ListBox::new();
    methods.set_selection_mode(gtk::SelectionMode::None);
    methods.add_css_class("boxed-list");

    let separate = gtk::CheckButton::new();
    let archive = gtk::CheckButton::new();
    archive.set_group(Some(&separate));
    separate.set_active(selection.send_method() == SendMethod::Separate);
    archive.set_active(selection.send_method() == SendMethod::Archive);

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

    methods
}

fn build_archive_settings(
    settings: &ArchiveSettings,
    sender: ComponentSender<ContentPanel>,
) -> gtk::Box {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    let group = adw::PreferencesGroup::new();

    let name_row = adw::ActionRow::builder().title("归档名称").build();
    let name_entry = gtk::Entry::builder()
        .text(&settings.archive_name)
        .hexpand(true)
        .width_chars(28)
        .valign(gtk::Align::Center)
        .build();
    name_entry.connect_changed({
        let sender = sender.clone();
        move |entry| {
            sender
                .output(ContentPanelOutput::ChangeArchiveName(
                    entry.text().to_string(),
                ))
                .ok();
        }
    });
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
        ArchiveFormat::Tar => 0,
        ArchiveFormat::TarZst => 1,
        ArchiveFormat::TarGz => 2,
        ArchiveFormat::Zip => 3,
    });
    format_row.connect_selected_notify({
        let sender = sender.clone();
        move |row| {
            let format = match row.selected() {
                0 => ArchiveFormat::Tar,
                1 => ArchiveFormat::TarZst,
                2 => ArchiveFormat::TarGz,
                3 => ArchiveFormat::Zip,
                _ => unreachable!("archive format row only exposes four options"),
            };
            sender
                .output(ContentPanelOutput::ChangeArchiveFormat(format))
                .ok();
        }
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
    compression.connect_active_name_notify({
        let sender = sender.clone();
        move |group| {
            let level = match group.active_name().as_deref() {
                Some("fast") => CompressionLevel::Fast,
                Some("balanced") => CompressionLevel::Balanced,
                Some("smaller") => CompressionLevel::Smaller,
                _ => unreachable!("compression group only exposes three named toggles"),
            };
            sender
                .output(ContentPanelOutput::ChangeArchiveCompression(level))
                .ok();
        }
    });
    compression_row.add_suffix(&compression);
    group.add(&compression_row);
    content.append(&group);

    let advanced = adw::ExpanderRow::builder().title("更多打包选项").build();
    advanced.add_row(&check_row(
        "包含所选文件夹本身",
        Some("关闭后只把文件夹中的内容放入归档根目录。"),
        settings.include_selected_folder,
        ArchiveOption::IncludeSelectedFolder,
        sender.clone(),
    ));
    advanced.add_row(&check_row(
        "包含隐藏文件",
        None,
        settings.include_hidden_files,
        ArchiveOption::IncludeHiddenFiles,
        sender.clone(),
    ));
    advanced.add_row(&check_row(
        "跟随符号链接",
        None,
        settings.follow_symlinks,
        ArchiveOption::FollowSymlinks,
        sender,
    ));
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
        let detail = match file {
            ContentItem::File { .. } => "文件".to_owned(),
            ContentItem::Folder { file_count, .. } => format!("{file_count} 个文件"),
        };
        let row = adw::ActionRow::builder()
            .title(file.name())
            .subtitle(&detail)
            .build();
        let icon_name = match file {
            ContentItem::File { .. } => "text-x-generic-symbolic",
            ContentItem::Folder { .. } => "folder-symbolic",
        };
        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(20);
        row.add_prefix(&icon);

        if let Some(size_bytes) = file.size_bytes() {
            let size = gtk::Label::new(Some(&format_size(size_bytes)));
            size.add_css_class("dim-label");
            size.set_valign(gtk::Align::Center);
            row.add_suffix(&size);
        }

        let remove = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .tooltip_text("移除此项目")
            .valign(gtk::Align::Center)
            .build();
        remove.add_css_class("flat");
        let item_path = file.path().to_owned();
        remove.connect_clicked({
            let sender = sender.clone();
            move |_| {
                sender
                    .output(ContentPanelOutput::RemoveItem(item_path.clone()))
                    .ok();
            }
        });
        row.add_suffix(&remove);
        list.append(&row);
    }

    list
}

fn check_row(
    title: &str,
    subtitle: Option<&str>,
    active: bool,
    option: ArchiveOption,
    sender: ComponentSender<ContentPanel>,
) -> adw::ActionRow {
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
    check.connect_toggled(move |check| {
        sender
            .output(ContentPanelOutput::ChangeArchiveOption(
                option,
                check.is_active(),
            ))
            .ok();
    });
    row.add_suffix(&check);
    row.set_activatable_widget(Some(&check));
    row
}
