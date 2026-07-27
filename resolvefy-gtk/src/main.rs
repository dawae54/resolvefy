mod ui;

use libadwaita::gio::prelude::ApplicationExtManual;

fn main() {
    ui::build_ui().run();
}
