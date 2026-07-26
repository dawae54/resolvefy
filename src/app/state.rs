use std::path::PathBuf;

use crate::converter::InputInfo;

pub struct ProgressState {
    pub progress: f32,
    pub status: String,
    pub done: bool,
    pub error: Option<String>,
}

pub struct AppState {
    pub input_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub input_info: Option<InputInfo>,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            progress: 0.0,
            status: String::new(),
            done: false,
            error: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: None,
            input_info: None,
        }
    }
}
