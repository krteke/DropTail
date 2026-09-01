use std::path::PathBuf;

use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::glib;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, adw, gtk,
};

use crate::application::{Application, discover_devices};
use crate::components::content_panel::{ContentPanel, ContentPanelMsg, ContentPanelOutput};
use crate::components::device_panel::{DevicePanel, DevicePanelMsg, DevicePanelOutput};
use crate::components::preferences_dialog::PreferencesDialog;
use crate::components::progress_panel::{ProgressPanel, ProgressPanelMsg};
use crate::components::shortcuts_dialog::ShortcutsDialog;
use crate::domain::content::{
    ArchiveFormat, ArchiveOption, CompressionLevel, ContentItem, SendMethod,
};
use crate::domain::device::DeviceList;
use crate::domain::preferences::PreferenceChange;
use crate::file_selection::{self, FileSelectionError};
use crate::presentation::{format_size, send_method_label};
use crate::transfer::{self, TransferEvent};

pub struct App {
    window: adw::ApplicationWindow,
    state: Application,
    device_panel: Controller<DevicePanel>,
    content_panel: Controller<ContentPanel>,
    preferences: Controller<PreferencesDialog>,
    progress_panel: Controller<ProgressPanel>,
    shortcuts_dialog: Controller<ShortcutsDialog>,
}

#[derive(Debug)]
pub enum AppMsg {
    DeviceSelected(String),
    RefreshDevices,
    AddFiles,
    AddFolder,
    FilesSelected(Vec<PathBuf>),
    FolderSelected(PathBuf),
    SelectionDialogFailed(String),
    SendMethodRequested(SendMethod),
    ArchiveFormatRequested(ArchiveFormat),
    ArchiveNameChanged(String),
    ArchiveCompressionChanged(CompressionLevel),
    ArchiveOptionChanged(ArchiveOption, bool),
    RemoveContentItem(PathBuf),
    PreferenceChanged(PreferenceChange),
    PrimaryAction,
    ShowPreferences,
    ShowShortcuts,
    ShowAbout,
    CloseRequest,
    ConfirmQuit,
}

#[derive(Debug)]
pub enum AppCommandOutput {
    ContentInspected(Result<Vec<ContentItem>, FileSelectionError>),
    DevicesDiscovered(Result<DeviceList, String>),
    Transfer(TransferEvent),
}

impl App {
    fn is_transferring(&self) -> bool {
        self.state.transfer().is_some()
    }

    fn page_name(&self) -> &'static str {
        if self.is_transferring() {
            "progress"
        } else {
            "compose"
        }
    }

    fn footer_title(&self) -> String {
        if let Some(transfer) = self.state.transfer() {
            if transfer.is_cancelling() {
                return format!("正在停止 {} 个文件的发送", transfer.item_count());
            }
            return format!("正在发送 {} 个文件", transfer.item_count());
        }

        let content = self.state.content();
        let target_name = self
            .state
            .selected_device()
            .map(|device| device.name.as_str());
        match (content.is_empty(), content.total_size_bytes(), target_name) {
            (true, _, Some(target)) => format!("尚未选择内容 → {target}"),
            (true, _, None) => "尚未选择内容".to_owned(),
            (false, Some(size), Some(target)) => format!(
                "{} 个项目 · {} → {target}",
                content.item_count(),
                format_size(size)
            ),
            (false, None, Some(target)) => {
                format!("{} 个项目 → {target}", content.item_count())
            }
            (false, Some(size), None) => {
                format!("{} 个项目 · {}", content.item_count(), format_size(size))
            }
            (false, None, None) => format!("{} 个项目", content.item_count()),
        }
    }

    fn footer_subtitle(&self) -> &str {
        if self.is_transferring() {
            if self
                .state
                .transfer()
                .expect("a transfer must exist while transferring")
                .is_cancelling()
            {
                "正在等待当前请求停止"
            } else {
                "关闭窗口前应确认是否停止当前传输"
            }
        } else {
            send_method_label(self.state.content().send_method())
        }
    }

    fn refresh_content_panel(&self) {
        self.content_panel
            .emit(ContentPanelMsg::Show(self.state.content().clone()));
    }

    fn choose_files(&self, sender: ComponentSender<Self>) {
        let dialog = gtk::FileDialog::builder()
            .title("选择要发送的文件")
            .accept_label("选择")
            .modal(true)
            .build();
        let window = self.window.clone();
        drop(relm4::spawn_local(async move {
            match dialog.open_multiple_future(Some(&window)).await {
                Ok(files) => match local_paths(&files) {
                    Some(paths) => sender.input(AppMsg::FilesSelected(paths)),
                    None => sender.input(AppMsg::SelectionDialogFailed(
                        "当前只能选择本地文件。".to_owned(),
                    )),
                },
                Err(error) if dialog_was_cancelled(&error) => {}
                Err(error) => {
                    sender.input(AppMsg::SelectionDialogFailed(error.to_string()));
                }
            }
        }));
    }

    fn choose_folder(&self, sender: ComponentSender<Self>) {
        let dialog = gtk::FileDialog::builder()
            .title("选择要发送的文件夹")
            .accept_label("选择")
            .modal(true)
            .build();
        let window = self.window.clone();
        drop(relm4::spawn_local(async move {
            match dialog.select_folder_future(Some(&window)).await {
                Ok(folder) => match folder.path() {
                    Some(path) => sender.input(AppMsg::FolderSelected(path)),
                    None => sender.input(AppMsg::SelectionDialogFailed(
                        "当前只能选择本地文件夹。".to_owned(),
                    )),
                },
                Err(error) if dialog_was_cancelled(&error) => {}
                Err(error) => {
                    sender.input(AppMsg::SelectionDialogFailed(error.to_string()));
                }
            }
        }));
    }

    fn show_error(&self, title: &str, message: &str) {
        let dialog = adw::AlertDialog::new(Some(title), Some(message));
        dialog.add_response("close", "关闭");
        dialog.set_close_response("close");
        dialog.present(Some(&self.window));
    }
}

#[relm4::component(pub)]
impl Component for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = AppCommandOutput;

    view! {
        #[root]
        main_window = adw::ApplicationWindow {
            set_title: Some("DropTail"),
            set_default_width: 1080,
            set_default_height: 780,
            set_width_request: 360,
            set_height_request: 500,

            #[wrap(Some)]
            set_content = &adw::ToolbarView {
                set_bottom_bar_style: adw::ToolbarStyle::Raised,

                add_top_bar = &adw::HeaderBar {
                    pack_start = &gtk::Button {
                        set_tooltip_text: Some("添加文件"),
                        #[watch]
                        set_sensitive: !model.is_transferring(),
                        connect_clicked => AppMsg::AddFiles,

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 6,

                            gtk::Image {
                                set_icon_name: Some("list-add-symbolic"),
                            },

                            #[name = "add_label"]
                            gtk::Label {
                                set_label: "添加",
                            },
                        },
                    },

                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "DropTail",
                    },

                    pack_end = &gtk::MenuButton {
                        set_icon_name: "open-menu-symbolic",
                        set_tooltip_text: Some("主菜单"),

                        #[wrap(Some)]
                        set_popover = &gtk::PopoverMenu::from_model(Some(&main_menu)) {},
                    },
                },

                #[name = "page_stack"]
                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                    set_transition_duration: 220,
                    set_vhomogeneous: false,

                    add_named: (&compose_scroll, Some("compose")),
                    add_named: (model.progress_panel.widget(), Some("progress")),

                    #[watch]
                    set_visible_child_name: model.page_name(),
                },

                #[name = "footer_box"]
                add_bottom_bar = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 16,
                    set_margin_top: 14,
                    set_margin_bottom: 14,
                    set_margin_start: 18,
                    set_margin_end: 18,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 2,
                        set_hexpand: true,
                        set_valign: gtk::Align::Center,

                        gtk::Label {
                            #[watch]
                            set_label: &model.footer_title(),
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.0,
                            set_wrap: true,
                            add_css_class: "heading",
                        },

                        gtk::Label {
                            #[watch]
                            set_label: model.footer_subtitle(),
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.0,
                            set_wrap: true,
                            add_css_class: "dim-label",
                        },
                    },

                    #[name = "primary_button"]
                    gtk::Button {
                        set_width_request: 136,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::End,

                        #[watch]
                        set_label: match model.state.transfer() {
                            Some(transfer) if transfer.is_cancelling() => "正在停止…",
                            Some(_) => "停止发送",
                            None => "发送",
                        },
                        #[watch]
                        set_sensitive: match model.state.transfer() {
                            Some(transfer) => !transfer.is_cancelling(),
                            None => !model.state.content().is_empty()
                                && model.state.selected_device().is_some(),
                        },
                        #[watch]
                        set_css_classes: if model.is_transferring() {
                            &["destructive-action"]
                        } else {
                            &["suggested-action"]
                        },
                        connect_clicked => AppMsg::PrimaryAction,
                    },
                },
            },

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::CloseRequest);
                glib::Propagation::Stop
            },
        }
    }

    menu! {
        main_menu: {
            "首选项" => PreferencesAction,
            "键盘快捷键" => ShortcutsAction,
            "关于 DropTail" => AboutAction,
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let state = Application::new();
        let preferences_data = state.preferences();
        let visible_devices = state.visible_devices(preferences_data.show_offline_devices);
        let content = state.content().clone();

        let device_panel = DevicePanel::builder().launch(visible_devices).forward(
            sender.input_sender(),
            |output| match output {
                DevicePanelOutput::Selected(device_id) => AppMsg::DeviceSelected(device_id),
                DevicePanelOutput::RefreshRequested => AppMsg::RefreshDevices,
            },
        );
        let content_panel = ContentPanel::builder().launch(content.clone()).forward(
            sender.input_sender(),
            |output| match output {
                ContentPanelOutput::AddFiles => AppMsg::AddFiles,
                ContentPanelOutput::AddFolder => AppMsg::AddFolder,
                ContentPanelOutput::ChangeSendMethod(method) => AppMsg::SendMethodRequested(method),
                ContentPanelOutput::ChangeArchiveFormat(format) => {
                    AppMsg::ArchiveFormatRequested(format)
                }
                ContentPanelOutput::ChangeArchiveName(name) => AppMsg::ArchiveNameChanged(name),
                ContentPanelOutput::ChangeArchiveCompression(compression) => {
                    AppMsg::ArchiveCompressionChanged(compression)
                }
                ContentPanelOutput::ChangeArchiveOption(option, active) => {
                    AppMsg::ArchiveOptionChanged(option, active)
                }
                ContentPanelOutput::RemoveItem(item_id) => AppMsg::RemoveContentItem(item_id),
            },
        );
        let preferences = PreferencesDialog::builder()
            .launch(preferences_data)
            .forward(sender.input_sender(), AppMsg::PreferenceChanged);
        let progress_panel = ProgressPanel::builder().launch(()).detach();
        let shortcuts_dialog = ShortcutsDialog::builder().launch(()).detach();

        let model = Self {
            window: root.clone(),
            state,
            device_panel,
            content_panel,
            preferences,
            progress_panel,
            shortcuts_dialog,
        };

        let compose_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        compose_box.append(model.device_panel.widget());
        compose_box.append(model.content_panel.widget());
        let compose_clamp = adw::Clamp::builder()
            .maximum_size(1180)
            .tightening_threshold(900)
            .child(&compose_box)
            .build();
        let compose_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&compose_clamp)
            .build();

        let widgets = view_output!();

        widgets.page_stack.set_visible_child_name("compose");

        let app = relm4::main_application();
        app.set_accelerators_for_action::<AddFilesAction>(&["<primary>O"]);
        app.set_accelerators_for_action::<AddFolderAction>(&["<primary><shift>O"]);
        app.set_accelerators_for_action::<PreferencesAction>(&["<primary>comma"]);

        let add_files = RelmAction::<AddFilesAction>::new_stateless({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::AddFiles)
        });
        let add_folder = RelmAction::<AddFolderAction>::new_stateless({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::AddFolder)
        });
        let preferences = RelmAction::<PreferencesAction>::new_stateless({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowPreferences)
        });
        let shortcuts = RelmAction::<ShortcutsAction>::new_stateless({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowShortcuts)
        });
        let about = RelmAction::<AboutAction>::new_stateless({
            let sender = sender.clone();
            move |_| sender.input(AppMsg::ShowAbout)
        });
        let mut actions = RelmActionGroup::<WindowActionGroup>::new();
        actions.add_action(add_files);
        actions.add_action(add_folder);
        actions.add_action(preferences);
        actions.add_action(shortcuts);
        actions.add_action(about);
        actions.register_for_widget(&widgets.main_window);

        let compact = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            560.0,
            adw::LengthUnit::Sp,
        ));
        compact.add_setter(
            &widgets.footer_box,
            "orientation",
            Some(&gtk::Orientation::Vertical.to_value()),
        );
        compact.add_setter(
            &widgets.primary_button,
            "halign",
            Some(&gtk::Align::Fill.to_value()),
        );
        compact.add_setter(
            &widgets.primary_button,
            "width-request",
            Some(&(-1_i32).to_value()),
        );
        compact.add_setter(&widgets.add_label, "visible", Some(&false.to_value()));
        for panel in [
            model.device_panel.widget(),
            model.content_panel.widget(),
            model.progress_panel.widget(),
        ] {
            compact.add_setter(panel, "margin-start", Some(&12_i32.to_value()));
            compact.add_setter(panel, "margin-end", Some(&12_i32.to_value()));
        }
        widgets.main_window.add_breakpoint(compact);

        model.device_panel.emit(DevicePanelMsg::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AppMsg::DeviceSelected(device_id) => {
                self.state.select_device(&device_id);
            }
            AppMsg::RefreshDevices => {
                sender.spawn_oneshot_command(|| {
                    AppCommandOutput::DevicesDiscovered(
                        discover_devices().map_err(|error| error.to_string()),
                    )
                });
            }
            AppMsg::AddFiles => self.choose_files(sender),
            AppMsg::AddFolder => self.choose_folder(sender),
            AppMsg::FilesSelected(paths) => {
                sender.spawn_oneshot_command(move || {
                    AppCommandOutput::ContentInspected(file_selection::inspect_files(paths))
                });
            }
            AppMsg::FolderSelected(path) => {
                sender.spawn_oneshot_command(move || {
                    AppCommandOutput::ContentInspected(file_selection::inspect_folder(path))
                });
            }
            AppMsg::SelectionDialogFailed(message) => {
                self.show_error("无法添加所选内容", &message);
            }
            AppMsg::SendMethodRequested(method) => {
                self.state.set_send_method(method);
                self.refresh_content_panel();
            }
            AppMsg::ArchiveFormatRequested(format) => {
                self.state.set_archive_format(format);
                self.refresh_content_panel();
            }
            AppMsg::ArchiveNameChanged(name) => {
                self.state.set_archive_name(name);
            }
            AppMsg::ArchiveCompressionChanged(compression) => {
                self.state.set_archive_compression(compression);
            }
            AppMsg::ArchiveOptionChanged(option, active) => {
                self.state.set_archive_option(option, active);
            }
            AppMsg::RemoveContentItem(path) => {
                self.state.remove_content(&path);
                self.refresh_content_panel();
            }
            AppMsg::PreferenceChanged(change) => {
                let show_offline = match change {
                    PreferenceChange::ShowOfflineDevices(value) => Some(value),
                    _ => None,
                };
                match self.state.update_preference(change) {
                    Ok(()) => {
                        if let Some(show_offline) = show_offline {
                            self.device_panel.emit(DevicePanelMsg::Show(
                                self.state.visible_devices(show_offline),
                            ));
                        }
                    }
                    Err(error) => self.show_error("首选项未保存", &error.to_string()),
                }
            }
            AppMsg::PrimaryAction
                if !self.is_transferring()
                    && !self.state.content().is_empty()
                    && self.state.selected_device().is_some() =>
            {
                let task = self.state.start_transfer();
                self.progress_panel.emit(ProgressPanelMsg::Show(Box::new(
                    self.state
                        .transfer()
                        .expect("the transfer was started immediately above")
                        .clone(),
                )));
                sender.spawn_command(move |output| {
                    transfer::run(task, move |event| {
                        output.emit(AppCommandOutput::Transfer(event));
                    });
                });
            }
            AppMsg::PrimaryAction if self.is_transferring() => {
                self.state.cancel_transfer();
                self.progress_panel.emit(ProgressPanelMsg::Show(Box::new(
                    self.state
                        .transfer()
                        .expect("cancellation keeps the transfer active until the request stops")
                        .clone(),
                )));
            }
            AppMsg::PrimaryAction => {}
            AppMsg::ShowPreferences => {
                self.preferences.widget().present(Some(&self.window));
            }
            AppMsg::ShowShortcuts => {
                self.shortcuts_dialog.widget().present(Some(&self.window));
            }
            AppMsg::ShowAbout => {
                adw::AboutDialog::builder()
                    .application_name("DropTail")
                    .application_icon("send-to-symbolic")
                    .version(env!("CARGO_PKG_VERSION"))
                    .comments("")
                    .license_type(gtk::License::Gpl30)
                    .build()
                    .present(Some(&self.window));
            }
            AppMsg::CloseRequest if !self.is_transferring() => {
                relm4::main_application().quit();
            }
            AppMsg::CloseRequest
                if self
                    .state
                    .transfer()
                    .expect("a transfer must exist while transferring")
                    .is_cancelling() =>
            {
                relm4::main_application().quit();
            }
            AppMsg::CloseRequest => {
                let item_count = self
                    .state
                    .transfer()
                    .expect("a close confirmation requires an active transfer")
                    .item_count();
                let body =
                    format!("当前传输尚未完成。关闭窗口会停止这 {item_count} 个文件的发送。");
                let dialog = adw::AlertDialog::new(Some("停止发送并退出？"), Some(&body));
                dialog.add_response("continue", "继续发送");
                dialog.add_response("quit", "停止并退出");
                dialog.set_close_response("continue");
                dialog.set_response_appearance("quit", adw::ResponseAppearance::Destructive);
                dialog.connect_response(None, move |_, response| {
                    if response == "quit" {
                        sender.input(AppMsg::ConfirmQuit);
                    }
                });
                dialog.present(Some(&self.window));
            }
            AppMsg::ConfirmQuit => {
                if self.is_transferring() {
                    self.state.cancel_transfer();
                }
                relm4::main_application().quit();
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AppCommandOutput::ContentInspected(Ok(items)) => {
                self.state.add_content(items);
                self.refresh_content_panel();
            }
            AppCommandOutput::ContentInspected(Err(error)) => {
                self.show_error("无法添加所选内容", &error.to_string());
            }
            AppCommandOutput::DevicesDiscovered(result) => {
                self.device_panel.emit(DevicePanelMsg::RefreshFinished);
                match result {
                    Ok(devices) => {
                        self.state.replace_devices(devices);
                        let show_offline = self.state.preferences().show_offline_devices;
                        self.device_panel.emit(DevicePanelMsg::Show(
                            self.state.visible_devices(show_offline),
                        ));
                    }
                    Err(error) => self.show_error("无法读取 Tailscale 设备", &error),
                }
            }
            AppCommandOutput::Transfer(TransferEvent::Sample {
                id,
                item_index,
                transferred_bytes,
                bytes_per_second,
            }) => {
                if self.state.record_transfer_sample(
                    id,
                    item_index,
                    transferred_bytes,
                    bytes_per_second,
                ) {
                    self.progress_panel.emit(ProgressPanelMsg::Show(Box::new(
                        self.state
                            .transfer()
                            .expect("progress requires an active transfer")
                            .clone(),
                    )));
                }
            }
            AppCommandOutput::Transfer(TransferEvent::ItemFinished { id, item_index }) => {
                if self.state.finish_transfer_item(id, item_index) {
                    self.progress_panel.emit(ProgressPanelMsg::Show(Box::new(
                        self.state
                            .transfer()
                            .expect("a finished item still belongs to an active transfer")
                            .clone(),
                    )));
                }
            }
            AppCommandOutput::Transfer(TransferEvent::Finished { id })
            | AppCommandOutput::Transfer(TransferEvent::Cancelled { id }) => {
                if self.state.end_transfer(id) {
                    self.progress_panel.emit(ProgressPanelMsg::Clear);
                }
            }
            AppCommandOutput::Transfer(TransferEvent::Failed { id, error }) => {
                if self.state.end_transfer(id) {
                    self.progress_panel.emit(ProgressPanelMsg::Clear);
                    self.show_error("发送失败", &error.to_string());
                }
            }
        }
    }
}

fn local_paths(files: &gtk::gio::ListModel) -> Option<Vec<PathBuf>> {
    (0..files.n_items())
        .map(|index| {
            let file = files
                .item(index)
                .expect("file dialog list indices must be valid")
                .downcast::<gtk::gio::File>()
                .expect("file dialog results must contain GFile objects");
            file.path()
        })
        .collect()
}

fn dialog_was_cancelled(error: &glib::Error) -> bool {
    error.matches(gtk::DialogError::Cancelled) || error.matches(gtk::DialogError::Dismissed)
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(AddFilesAction, WindowActionGroup, "add-files");
relm4::new_stateless_action!(AddFolderAction, WindowActionGroup, "add-folder");
relm4::new_stateless_action!(PreferencesAction, WindowActionGroup, "preferences");
relm4::new_stateless_action!(ShortcutsAction, WindowActionGroup, "shortcuts");
relm4::new_stateless_action!(AboutAction, WindowActionGroup, "about");
