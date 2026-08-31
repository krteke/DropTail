mod app;
mod components;

use relm4::RelmApp;

fn main() {
    let app = RelmApp::new("io.github.droptail.Send");
    app.run::<app::App>(());
}
