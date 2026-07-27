use std::path::PathBuf;

use crate::converter::{EncodeMode, InputInfo};

pub struct ProgressState {
    pub progress: f64,
    pub status: String,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct AppState {
    pub input_path: Option<PathBuf>,
    pub output_path: Option<PathBuf>,
    pub input_info: Option<InputInfo>,
    pub encode_mode: EncodeMode,
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
