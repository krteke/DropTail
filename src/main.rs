mod app;
mod application;
mod archive;
mod components;
mod domain;
mod file_selection;
mod presentation;
mod settings;
mod tailscale;
mod transfer;

use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("io.github.krteke.DropTail");
    app.run::<app::App>(());
}
