mod app;
mod application;
mod components;
mod domain;
mod file_selection;
mod mock_api;
mod presentation;
mod settings;

use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("io.github.krteke.DropTail");
    app.run::<app::App>(());
}
