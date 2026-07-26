mod app;
mod converter;
mod ui;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::gio::prelude::ApplicationExtManual;

use app::{AppState, ProgressState};

fn main() {
    let state = Rc::new(RefCell::new(AppState::default()));
    let progress_state = Arc::new(Mutex::new(ProgressState::default()));

    let app = ui::build_ui(state, progress_state);
    app.run();
}
