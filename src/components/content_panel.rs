use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentMode {
    Empty,
    Files,
    PackedFiles,
    Folder,
}

impl ContentMode {
    fn stack_name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Files => "files",
            Self::PackedFiles => "packed-files",
            Self::Folder => "folder",
        }
    }

    fn summary(self) -> ContentSummary {
        match self {
            Self::Empty => ContentSummary::EMPTY,
            Self::Files => ContentSummary {
                items: 3,
                size: "1.83 GiB",
                method: "分别发送",
            },
            Self::PackedFiles => ContentSummary {
                items: 3,
                size: "1.83 GiB",
                method: "打包后发送",
            },
            Self::Folder => ContentSummary {
                items: 2,
                size: "1.33 GiB",
                method: "打包后发送",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentSummary {
    pub items: u8,
    pub size: &'static str,
    pub method: &'static str,
}

impl ContentSummary {
    pub const EMPTY: Self = Self {
        items: 0,
        size: "",
        method: "添加文件后即可发送",
    };

    pub fn is_ready(self) -> bool {
        self.items > 0
    }

    pub fn description(self) -> String {
        if self.is_ready() {
            format!("{} 个项目 · {}", self.items, self.size)
        } else {
            "可拖放，也可通过文件选择器添加".to_owned()
        }
    }
}

#[derive(Debug)]
pub enum ContentPanelMsg {
    PreviewFiles,
    PreviewFolder,
    PackFiles,
    Reset,
}

#[derive(Debug)]
pub enum ContentPanelOutput {
    SummaryChanged(ContentSummary),
}

pub struct ContentPanel {
    mode: ContentMode,
}

#[relm4::component(pub)]
impl SimpleComponent for ContentPanel {
    type Init = ();
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
                            set_label: &model.mode.summary().description(),
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
                        set_visible: model.mode != ContentMode::Empty,

                        gtk::Button {
                            set_label: "添加文件",
                            set_tooltip_text: Some("添加文件"),
                            connect_clicked => ContentPanelMsg::PreviewFiles,
                        },

                        gtk::Button {
                            set_label: "添加文件夹",
                            set_tooltip_text: Some("添加文件夹"),
                            connect_clicked => ContentPanelMsg::PreviewFolder,
                        },
                },
            },

            #[name = "content_stack"]
            gtk::Stack {
                set_transition_type: gtk::StackTransitionType::Crossfade,
                set_transition_duration: 180,
                set_vhomogeneous: false,

                add_named: (&empty_page, Some("empty")),
                add_named: (&files_page, Some("files")),
                add_named: (&packed_files_page, Some("packed-files")),
                add_named: (&folder_page, Some("folder")),

                #[watch]
                set_visible_child_name: model.mode.stack_name(),
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            mode: ContentMode::Empty,
        };
        let empty_page = build_empty_page(sender.clone());
        let files_page = build_files_page(sender.clone());
        let packed_files_page = build_archive_page(sender.clone(), ArchivePreview::Files);
        let folder_page = build_archive_page(sender.clone(), ArchivePreview::Folder);
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.mode = match msg {
            // TODO(integration): replace the fixtures with paths returned by a file chooser.
            ContentPanelMsg::PreviewFiles => ContentMode::Files,
            // TODO(integration): replace the fixture with a folder selected by the user.
            ContentPanelMsg::PreviewFolder => ContentMode::Folder,
            ContentPanelMsg::PackFiles => ContentMode::PackedFiles,
            ContentPanelMsg::Reset => ContentMode::Empty,
        };
        sender
            .output(ContentPanelOutput::SummaryChanged(self.mode.summary()))
            .ok();
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
        move |_| sender.input(ContentPanelMsg::PreviewFiles)
    });
    let folder_button = gtk::Button::with_label("选择文件夹…");
    folder_button.set_tooltip_text(Some("选择文件夹"));
    folder_button.connect_clicked(move |_| sender.input(ContentPanelMsg::PreviewFolder));
    actions.append(&file_button);
    actions.append(&folder_button);

    content.append(&icon);
    content.append(&title);
    content.append(&subtitle);
    content.append(&actions);
    frame.set_child(Some(&content));
    frame
}

fn build_files_page(sender: ComponentSender<ContentPanel>) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(26)
        .build();

    let files = [
        FilePreview::file("presentation.pdf", "17.5 MiB"),
        FilePreview::file("dataset.csv.zst", "652 MiB"),
        FilePreview::file("recording.mkv", "1.17 GiB"),
    ];
    page.append(&build_file_list(&files, sender.clone()));

    let method_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    method_box.append(&section_heading("发送方式", "只在选择内容需要决策时出现"));

    let methods = gtk::ListBox::new();
    methods.set_selection_mode(gtk::SelectionMode::None);
    methods.add_css_class("boxed-list");

    let separate = gtk::CheckButton::new();
    separate.set_active(true);
    let separate_row = adw::ActionRow::builder()
        .title("分别发送")
        .subtitle(
            "每个文件都是独立的 Taildrop 传输。单个文件失败不会影响其他文件，接收端也不需要解压。",
        )
        .activatable(true)
        .build();
    separate_row.add_prefix(&separate);
    separate_row.set_activatable_widget(Some(&separate));
    methods.append(&separate_row);

    let packed = gtk::CheckButton::new();
    packed.set_group(Some(&separate));
    let packed_row = adw::ActionRow::builder()
        .title("打成一个包")
        .subtitle("接收端只得到一个归档文件，适合希望把这一批内容作为一个整体处理时。")
        .activatable(true)
        .build();
    packed_row.add_prefix(&packed);
    packed_row.set_activatable_widget(Some(&packed));
    packed.connect_toggled(move |button| {
        if button.is_active() {
            sender.input(ContentPanelMsg::PackFiles);
        }
    });
    methods.append(&packed_row);
    method_box.append(&methods);
    page.append(&method_box);

    page
}

#[derive(Clone, Copy)]
enum ArchivePreview {
    Files,
    Folder,
}

fn build_archive_page(sender: ComponentSender<ContentPanel>, preview: ArchivePreview) -> gtk::Box {
    let page = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(20)
        .build();

    let files = match preview {
        ArchivePreview::Files => vec![
            FilePreview::file("presentation.pdf", "17.5 MiB"),
            FilePreview::file("dataset.csv.zst", "652 MiB"),
            FilePreview::file("recording.mkv", "1.17 GiB"),
        ],
        ArchivePreview::Folder => vec![
            FilePreview::folder("Project Assets/", "412 个项目", "1.33 GiB"),
            FilePreview::file("README.md", "17.6 KiB"),
        ],
    };
    page.append(&build_file_list(&files, sender));

    let settings = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    settings.append(&section_heading("发送方式", "只在选择内容需要决策时出现"));

    let banner_text = match preview {
        ArchivePreview::Files => "这些文件会先创建为一个归档，再把归档交给 Taildrop。",
        ArchivePreview::Folder => {
            "选择中包含文件夹。Taildrop 发送的是文件，因此这里会先创建一个归档，再把归档交给 Taildrop。"
        }
    };
    let banner = adw::Banner::new(banner_text);
    banner.set_revealed(true);
    settings.append(&banner);

    let group = adw::PreferencesGroup::new();

    let archive_name = match preview {
        ArchivePreview::Files => "Taildrop files.tar.zst",
        ArchivePreview::Folder => "Project Assets + 1 item.tar.zst",
    };
    let name_row = adw::ActionRow::builder()
        .title("归档名称")
        .subtitle("接收设备最终看到的文件名")
        .build();
    let name_entry = gtk::Entry::builder()
        .text(archive_name)
        .hexpand(true)
        .width_chars(28)
        .valign(gtk::Align::Center)
        .build();
    name_row.add_suffix(&name_entry);
    group.add(&name_row);

    let format_row = adw::ComboRow::builder()
        .title("归档格式")
        .subtitle("目标设备是 Linux，tar.zst 通常兼顾速度、权限信息和体积。")
        .build();
    let formats = gtk::StringList::new(&[
        "tar.zst · 推荐，快速压缩",
        "tar.gz · 通用兼容",
        "zip · 跨平台",
    ]);
    format_row.set_model(Some(&formats));
    format_row.set_selected(0);
    group.add(&format_row);

    let compression_row = adw::ActionRow::builder()
        .title("压缩级别")
        .subtitle("不暴露算法数字，统一成能理解的目标")
        .build();
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
    compression.add(adw::Toggle::builder().name("small").label("更小").build());
    compression.set_active_name(Some("balanced"));
    compression_row.add_suffix(&compression);
    group.add(&compression_row);
    settings.append(&group);

    let advanced = adw::ExpanderRow::builder().title("更多打包选项").build();
    advanced.add_row(&check_row(
        "包含所选文件夹本身",
        "关闭后只把文件夹中的内容放入归档根目录。",
        true,
    ));
    advanced.add_row(&check_row(
        "包含隐藏文件",
        "默认把所选目录当成一个完整目录，而不是替用户猜哪些内容“不重要”。",
        true,
    ));
    advanced.add_row(&check_row(
        "跟随符号链接",
        "默认关闭，避免无意把链接指向的大目录也打包进去。",
        false,
    ));
    let advanced_group = adw::PreferencesGroup::new();
    advanced_group.add(&advanced);
    settings.append(&advanced_group);

    let note = gtk::Label::new(Some(
        "准备归档时会显示单独进度；临时归档在发送结束或取消后自动清理。",
    ));
    note.set_halign(gtk::Align::Start);
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("caption");
    note.add_css_class("dim-label");
    settings.append(&note);

    page.append(&settings);
    page
}

fn section_heading(title: &str, subtitle: &str) -> gtk::Box {
    let heading = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .build();
    let title_label = gtk::Label::new(Some(title));
    title_label.set_halign(gtk::Align::Start);
    title_label.add_css_class("title-3");
    let subtitle_label = gtk::Label::new(Some(subtitle));
    subtitle_label.set_halign(gtk::Align::Start);
    subtitle_label.set_xalign(0.0);
    subtitle_label.set_wrap(true);
    subtitle_label.add_css_class("dim-label");
    heading.append(&title_label);
    heading.append(&subtitle_label);
    heading
}

#[derive(Clone, Copy)]
struct FilePreview {
    name: &'static str,
    detail: &'static str,
    size: &'static str,
    icon_name: &'static str,
}

impl FilePreview {
    const fn file(name: &'static str, size: &'static str) -> Self {
        Self {
            name,
            detail: "文件",
            size,
            icon_name: "text-x-generic-symbolic",
        }
    }

    const fn folder(name: &'static str, detail: &'static str, size: &'static str) -> Self {
        Self {
            name,
            detail,
            size,
            icon_name: "folder-symbolic",
        }
    }
}

fn build_file_list(files: &[FilePreview], sender: ComponentSender<ContentPanel>) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");

    for file in files {
        let row = adw::ActionRow::builder()
            .title(file.name)
            .subtitle(file.detail)
            .build();
        let icon = gtk::Image::from_icon_name(file.icon_name);
        icon.set_pixel_size(20);
        row.add_prefix(&icon);

        let size = gtk::Label::new(Some(file.size));
        size.add_css_class("dim-label");
        size.set_valign(gtk::Align::Center);
        row.add_suffix(&size);

        let remove = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .tooltip_text("移除此项目")
            .valign(gtk::Align::Center)
            .build();
        remove.add_css_class("flat");
        remove.connect_clicked({
            let sender = sender.clone();
            move |_| sender.input(ContentPanelMsg::Reset)
        });
        row.add_suffix(&remove);
        list.append(&row);
    }

    list
}

fn check_row(title: &str, subtitle: &str, active: bool) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(subtitle)
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
