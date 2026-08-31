use relm4::adw::prelude::*;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw, gtk};

use crate::models::{ConnectionKind, DeviceKind, TransferItemState, TransferSnapshot};
use crate::presentation::{format_rate, format_size};

#[derive(Debug)]
pub enum ProgressPanelMsg {
    Show(TransferSnapshot),
}

pub struct ProgressPanel {
    snapshot: Option<TransferSnapshot>,
}

#[relm4::component(pub)]
impl SimpleComponent for ProgressPanel {
    type Init = ();
    type Input = ProgressPanelMsg;
    type Output = ();

    view! {
        #[root]
        root = gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_top: 28,
            set_margin_bottom: 32,
            set_margin_start: 24,
            set_margin_end: 24,

            adw::Bin {
                #[watch]
                set_child: model.snapshot.as_ref().map(build_progress_content).as_ref(),
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { snapshot: None };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ProgressPanelMsg::Show(snapshot) => self.snapshot = Some(snapshot),
        }
    }
}

fn build_progress_content(snapshot: &TransferSnapshot) -> gtk::Box {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(24)
        .build();

    let target_list = gtk::ListBox::new();
    target_list.set_selection_mode(gtk::SelectionMode::None);
    target_list.add_css_class("boxed-list");

    let connection = match &snapshot.target.connection {
        ConnectionKind::Online => "在线".to_owned(),
        ConnectionKind::Offline => "离线".to_owned(),
    };
    let target_detail = format!("{} · {connection}", snapshot.target.address);
    let target_row = adw::ActionRow::builder()
        .title(&snapshot.target.name)
        .subtitle(&target_detail)
        .build();
    let icon_name = match snapshot.target.kind {
        DeviceKind::Computer => "computer-symbolic",
        DeviceKind::Phone => "phone-symbolic",
    };
    let target_icon = gtk::Image::from_icon_name(icon_name);
    target_icon.set_pixel_size(24);
    target_row.add_prefix(&target_icon);
    target_list.append(&target_row);
    content.append(&target_list);

    let progress_section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();
    progress_section.append(&section_heading());
    progress_section.append(&progress_card(snapshot));
    progress_section.append(&queue_list(snapshot));
    content.append(&progress_section);

    content
}

fn section_heading() -> gtk::Box {
    let heading = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .build();
    let title = gtk::Label::new(Some("正在发送"));
    title.set_halign(gtk::Align::Start);
    title.add_css_class("title-3");
    let subtitle = gtk::Label::new(Some("当前文件与总体进度"));
    subtitle.set_halign(gtk::Align::Start);
    subtitle.add_css_class("dim-label");
    heading.append(&title);
    heading.append(&subtitle);
    heading
}

fn progress_card(snapshot: &TransferSnapshot) -> gtk::Frame {
    let frame = gtk::Frame::new(None);
    frame.add_css_class("card");

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(14)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();
    let header = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    let current = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let filename = gtk::Label::new(Some(&snapshot.current_name));
    filename.set_halign(gtk::Align::Start);
    filename.add_css_class("heading");
    let status = gtk::Label::new(Some("Taildrop 正在发送"));
    status.set_halign(gtk::Align::Start);
    status.add_css_class("dim-label");
    current.append(&filename);
    current.append(&status);
    header.append(&current);

    let percentage_text = format!("{:.0}%", snapshot.progress * 100.0);
    let percentage = gtk::Label::new(Some(&percentage_text));
    percentage.set_valign(gtk::Align::Start);
    percentage.add_css_class("title-3");
    header.append(&percentage);
    content.append(&header);

    let progress = gtk::ProgressBar::new();
    progress.set_fraction(snapshot.progress);
    content.append(&progress);

    let details = adw::WrapBox::builder()
        .orientation(gtk::Orientation::Horizontal)
        .child_spacing(12)
        .line_spacing(4)
        .justify(adw::JustifyMode::Spread)
        .justify_last_line(true)
        .build();
    let transferred_text = format!(
        "{} / {}",
        format_size(snapshot.transferred_bytes),
        format_size(snapshot.total_bytes)
    );
    let transferred = gtk::Label::new(Some(&transferred_text));
    transferred.set_halign(gtk::Align::Start);
    transferred.add_css_class("dim-label");
    let rate_text = format!(
        "{}   约 {} 秒",
        format_rate(snapshot.bytes_per_second),
        snapshot.eta_seconds
    );
    let rate = gtk::Label::new(Some(&rate_text));
    rate.set_halign(gtk::Align::End);
    rate.add_css_class("dim-label");
    details.append(&transferred);
    details.append(&rate);
    content.append(&details);

    frame.set_child(Some(&content));
    frame
}

fn queue_list(snapshot: &TransferSnapshot) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");

    for item in &snapshot.queue {
        let row = adw::ActionRow::builder().title(&item.name).build();
        let state = match item.state {
            TransferItemState::Sending => "发送中",
            TransferItemState::Waiting => "等待",
        };
        let state_label = gtk::Label::new(Some(state));
        state_label.add_css_class("pill");
        row.add_suffix(&state_label);
        list.append(&row);
    }

    list
}
