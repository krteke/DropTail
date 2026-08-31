mod app;
mod application;
mod components;
mod mock_api;
mod models;
mod presentation;
mod settings;

use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("io.github.krteke.DropTail");
    app.run::<app::App>(());
}
