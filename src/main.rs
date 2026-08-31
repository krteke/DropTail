mod app;
mod application;
mod components;
mod mock_api;
mod models;
mod presentation;

use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("io.github.droptail.Send");
    app.run::<app::App>(());
}
