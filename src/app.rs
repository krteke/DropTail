use relm4::actions::{AccelsPlus, RelmAction, RelmActionGroup};
use relm4::adw::prelude::*;
use relm4::gtk::glib;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    adw, gtk,
};

use crate::application::{AppService, ContentRequest};
use crate::components::content_panel::{ContentPanel, ContentPanelMsg, ContentPanelOutput};
use crate::components::device_panel::{DevicePanel, DevicePanelOutput};
use crate::components::preferences_dialog::PreferencesDialog;
use crate::components::progress_panel::{ProgressPanel, ProgressPanelMsg};
use crate::models::{ContentPreview, Device, PreferenceChange, SendMethod, TransferSnapshot};
use crate::presentation::{format_size, send_method_label};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Compose,
    Progress,
}

impl Page {
    fn stack_name(self) -> &'static str {
        match self {
            Self::Compose => "compose",
            Self::Progress => "progress",
        }
    }
}

pub struct App {
    window: adw::ApplicationWindow,
    service: AppService,
    device_panel: Controller<DevicePanel>,
    content_panel: Controller<ContentPanel>,
    preferences: Controller<PreferencesDialog>,
    progress_panel: Controller<ProgressPanel>,
    page: Page,
    target: Device,
    content: ContentPreview,
    transfer: Option<TransferSnapshot>,
}

#[derive(Debug)]
pub enum AppMsg {
    DeviceSelected(Device),
    AddFiles,
    AddFolder,
    SendMethodRequested(SendMethod),
    RemoveContentItem(String),
    PreferenceChanged(PreferenceChange),
    PrimaryAction,
    ShowPreferences,
    ShowShortcuts,
    ShowAbout,
    CloseRequest,
    ConfirmQuit,
}

impl App {
    fn footer_title(&self) -> String {
        match self.page {
            Page::Progress => format!(
                "正在发送 {} 个文件",
                self.transfer
                    .as_ref()
                    .expect("progress page must have an active transfer")
                    .item_count()
            ),
            Page::Compose if self.content.summary.is_ready() => format!(
                "{} 个项目 · {} → {}",
                self.content.summary.item_count,
                format_size(self.content.summary.total_size_bytes),
                self.target.name
            ),
            Page::Compose => format!("尚未选择内容 → {}", self.target.name),
        }
    }

    fn footer_subtitle(&self) -> &str {
        match self.page {
            Page::Progress => "关闭窗口前应确认是否停止当前传输",
            Page::Compose => send_method_label(self.content.summary.method),
        }
    }

    fn show_content(&mut self, content: ContentPreview) {
        self.content_panel
            .emit(ContentPanelMsg::Show(content.clone()));
        self.content = content;
    }
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        #[root]
        main_window = adw::ApplicationWindow {
            set_title: Some("Taildrop Send"),
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
                        set_sensitive: model.page == Page::Compose,
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
                        set_title: "Taildrop Send",
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
                    set_visible_child_name: model.page.stack_name(),
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
                        set_label: if model.page == Page::Compose { "发送" } else { "停止发送" },
                        #[watch]
                        set_sensitive: model.page == Page::Progress || model.content.summary.is_ready(),
                        #[watch]
                        set_css_classes: if model.page == Page::Compose {
                            &["suggested-action"]
                        } else {
                            &["destructive-action"]
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
            "关于 Taildrop Send" => AboutAction,
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let service = AppService;
        let bootstrap = service.bootstrap();
        let target = bootstrap.selected_device;
        let content = bootstrap.content;

        let device_panel = DevicePanel::builder().launch(bootstrap.devices).forward(
            sender.input_sender(),
            |output| match output {
                DevicePanelOutput::Selected(device) => AppMsg::DeviceSelected(device),
            },
        );
        let content_panel = ContentPanel::builder().launch(content.clone()).forward(
            sender.input_sender(),
            |output| match output {
                ContentPanelOutput::AddFiles => AppMsg::AddFiles,
                ContentPanelOutput::AddFolder => AppMsg::AddFolder,
                ContentPanelOutput::ChangeSendMethod(method) => AppMsg::SendMethodRequested(method),
                ContentPanelOutput::RemoveItem(item_id) => AppMsg::RemoveContentItem(item_id),
            },
        );
        let preferences = PreferencesDialog::builder()
            .launch(bootstrap.preferences)
            .forward(sender.input_sender(), AppMsg::PreferenceChanged);
        let progress_panel = ProgressPanel::builder().launch(()).detach();

        let model = Self {
            window: root.clone(),
            service,
            device_panel,
            content_panel,
            preferences,
            progress_panel,
            page: Page::Compose,
            target,
            content,
            transfer: None,
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

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::DeviceSelected(device) => self.target = device,
            AppMsg::AddFiles => {
                let content = self.service.fetch_content(ContentRequest::Files);
                self.show_content(content);
            }
            AppMsg::AddFolder => {
                let content = self.service.fetch_content(ContentRequest::Folder);
                self.show_content(content);
            }
            AppMsg::SendMethodRequested(method) => {
                let content = self.service.fetch_content(ContentRequest::Method(method));
                self.show_content(content);
            }
            AppMsg::RemoveContentItem(item_id) => {
                let content = self.service.remove_content_item(&item_id);
                self.show_content(content);
            }
            AppMsg::PreferenceChanged(change) => self.service.update_preference(change),
            AppMsg::PrimaryAction
                if self.page == Page::Compose && self.content.summary.is_ready() =>
            {
                let transfer = self.service.start_transfer(&self.target, &self.content);
                self.progress_panel
                    .emit(ProgressPanelMsg::Show(transfer.clone()));
                self.transfer = Some(transfer);
                self.page = Page::Progress;
            }
            AppMsg::PrimaryAction if self.page == Page::Progress => {
                let transfer = self
                    .transfer
                    .take()
                    .expect("progress page must have an active transfer");
                self.service.cancel_transfer(&transfer.id);
                self.page = Page::Compose;
            }
            AppMsg::PrimaryAction => {}
            AppMsg::ShowPreferences => {
                self.preferences.widget().present(Some(&self.window));
            }
            AppMsg::ShowShortcuts => {
                let dialog = adw::AlertDialog::new(
                    Some("键盘快捷键"),
                    Some(
                        "Ctrl+O  添加文件\nCtrl+Shift+O  添加文件夹\nCtrl+,  打开首选项\nEsc  关闭当前对话框",
                    ),
                );
                dialog.add_response("close", "关闭");
                dialog.set_close_response("close");
                dialog.present(Some(&self.window));
            }
            AppMsg::ShowAbout => {
                adw::AboutDialog::builder()
                    .application_name("Taildrop Send")
                    .application_icon("send-to-symbolic")
                    .developer_name("DropTail")
                    .version(env!("CARGO_PKG_VERSION"))
                    .comments("一个专注于 Taildrop 发送流程的桌面界面原型。")
                    .license_type(gtk::License::MitX11)
                    .build()
                    .present(Some(&self.window));
            }
            AppMsg::CloseRequest if self.page == Page::Compose => {
                relm4::main_application().quit();
            }
            AppMsg::CloseRequest => {
                let item_count = self
                    .transfer
                    .as_ref()
                    .expect("progress page must have an active transfer")
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
                let transfer = self
                    .transfer
                    .take()
                    .expect("quit confirmation requires an active transfer");
                self.service.cancel_transfer(&transfer.id);
                relm4::main_application().quit();
            }
        }
    }
}

relm4::new_action_group!(WindowActionGroup, "win");
relm4::new_stateless_action!(AddFilesAction, WindowActionGroup, "add-files");
relm4::new_stateless_action!(AddFolderAction, WindowActionGroup, "add-folder");
relm4::new_stateless_action!(PreferencesAction, WindowActionGroup, "preferences");
relm4::new_stateless_action!(ShortcutsAction, WindowActionGroup, "shortcuts");
relm4::new_stateless_action!(AboutAction, WindowActionGroup, "about");
