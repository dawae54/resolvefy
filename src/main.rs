mod converter;
mod ui;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::gio::prelude::ApplicationExtManual;

struct ProgressState {
    progress: f32,
    status: String,
    done: bool,
    error: Option<String>,
}

struct State {
    input_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    input_info: Option<converter::InputInfo>,
}

fn main() {
    let state = Rc::new(RefCell::new(State {
        input_path: None,
        output_path: None,
        input_info: None,
    }));

    let progress_state = Arc::new(Mutex::new(ProgressState {
        progress: 0.0,
        status: String::new(),
        done: false,
        error: None,
    }));

    let app = ui::build_ui(state, progress_state);
    app.run();
}
