use relm4::gtk::gio::prelude::ApplicationExt;
use relm4::gtk::prelude::GtkApplicationExt;
use relm4::{adw, gtk};

const TRANSFER_NOTIFICATION_ID: &str = "transfer-finished";

pub struct DesktopIntegration {
    application: gtk::Application,
    suspend_inhibit_cookie: Option<u32>,
}

impl DesktopIntegration {
    pub fn new() -> Self {
        Self {
            application: relm4::main_application(),
            suspend_inhibit_cookie: None,
        }
    }

    pub fn inhibit_suspend(&mut self, window: &adw::ApplicationWindow) {
        if self.suspend_inhibit_cookie.is_some() {
            return;
        }

        let cookie = self.application.inhibit(
            Some(window),
            gtk::ApplicationInhibitFlags::SUSPEND,
            Some("正在准备归档或发送文件"),
        );
        if cookie != 0 {
            self.suspend_inhibit_cookie = Some(cookie);
        }
    }

    pub fn allow_suspend(&mut self) {
        if let Some(cookie) = self.suspend_inhibit_cookie.take() {
            self.application.uninhibit(cookie);
        }
    }

    pub fn notify_transfer_finished(&self, target_name: &str, item_count: usize) {
        let notification = gtk::gio::Notification::new("发送完成");
        notification.set_body(Some(&format!(
            "已向 {target_name} 发送 {item_count} 个文件。"
        )));
        self.application
            .send_notification(Some(TRANSFER_NOTIFICATION_ID), &notification);
    }
}

impl Drop for DesktopIntegration {
    fn drop(&mut self) {
        self.allow_suspend();
    }
}
