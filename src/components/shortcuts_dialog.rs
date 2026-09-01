use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw};

pub struct ShortcutsDialog;

#[relm4::component(pub)]
impl SimpleComponent for ShortcutsDialog {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        #[root]
        dialog = adw::ShortcutsDialog {
            set_title: "键盘快捷键",
            set_content_width: 640,
            set_content_height: 420,
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self;
        let widgets = view_output!();

        let section = adw::ShortcutsSection::new(None);
        section.add(adw::ShortcutsItem::from_action("添加文件", "win.add-files"));
        section.add(adw::ShortcutsItem::from_action(
            "添加文件夹",
            "win.add-folder",
        ));
        section.add(adw::ShortcutsItem::from_action(
            "打开首选项",
            "win.preferences",
        ));
        section.add(adw::ShortcutsItem::new("关闭对话框", "Escape"));
        root.add(section);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {}
}
