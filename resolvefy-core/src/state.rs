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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_state_default() {
        let state = ProgressState::default();
        assert_eq!(state.progress, 0.0);
        assert!(state.status.is_empty());
        assert!(!state.done);
        assert!(state.error.is_none());
    }

    #[test]
    fn progress_state_with_values() {
        let state = ProgressState {
            progress: 50.0,
            status: "converting".to_string(),
            done: false,
            error: None,
        };
        assert_eq!(state.progress, 50.0);
        assert_eq!(state.status, "converting");
        assert!(!state.done);
    }

    #[test]
    fn progress_state_done() {
        let state = ProgressState {
            progress: 100.0,
            status: "done".to_string(),
            done: true,
            error: None,
        };
        assert!(state.done);
        assert_eq!(state.progress, 100.0);
    }

    #[test]
    fn progress_state_with_error() {
        let state = ProgressState {
            progress: 0.0,
            status: "failed".to_string(),
            done: false,
            error: Some("ffmpeg error".to_string()),
        };
        assert!(state.error.is_some());
        assert_eq!(state.error.unwrap(), "ffmpeg error");
    }

    #[test]
    fn app_state_default() {
        let state = AppState::default();
        assert!(state.input_path.is_none());
        assert!(state.output_path.is_none());
        assert!(state.input_info.is_none());
        assert_eq!(state.encode_mode, EncodeMode::CRF);
    }

    #[test]
    fn app_state_with_paths() {
        let state = AppState {
            input_path: Some(PathBuf::from("/input.mp4")),
            output_path: Some(PathBuf::from("/output.mp4")),
            input_info: None,
            encode_mode: EncodeMode::CBR,
        };
        assert_eq!(state.input_path.unwrap(), PathBuf::from("/input.mp4"));
        assert_eq!(state.output_path.unwrap(), PathBuf::from("/output.mp4"));
        assert_eq!(state.encode_mode, EncodeMode::CBR);
    }
}
