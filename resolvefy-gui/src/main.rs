mod window;

use gtk4::glib;
use gtk4::prelude::*;

const APP_ID: &str = "io.github.dawae54.Resolvefy";

fn main() -> glib::ExitCode {
    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(window::build_ui);

    app.run()
}