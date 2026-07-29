use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use resolvefy_core::converter::{self, EncodeConfig, EncodeMode};
use resolvefy_core::{AppState, ProgressState};

slint::include_modules!();

fn parse_crf(text: &str) -> u32 {
    text.parse::<i32>()
        .map(|v| v.clamp(1, 63) as u32)
        .unwrap_or(24)
}

fn parse_bitrate(text: &str) -> u32 {
    text.parse::<u32>().map(|v| v.max(100)).unwrap_or(5000)
}

fn main() {
    let window = MainWindow::new().unwrap();
    let state = Rc::new(RefCell::new(AppState::default()));
    let progress_state = Arc::new(Mutex::new(ProgressState::default()));

    {
        let state = state.clone();
        let window_weak = window.as_weak();
        window.on_pick_input(move || {
            let Some(window) = window_weak.upgrade() else {
                return;
            };
            let dialog = rfd::FileDialog::new()
                .add_filter("Vídeo", &["mp4", "mkv", "avi", "mov", "webm", "flv", "ts"])
                .set_title("Seleccionar vídeo");
            let Some(path) = dialog.pick_file() else {
                return;
            };

            window.set_input_path(path.display().to_string().into());

            match converter::detect_input(&path) {
                Ok(info) => {
                    let mut s = state.borrow_mut();
                    let auto_out = converter::default_output_name(&path);
                    window.set_output_path(auto_out.display().to_string().into());

                    let video_hint = if info.is_video_av1 {
                        "→ copy"
                    } else {
                        "→ SVT-AV1"
                    };
                    let audio_hint = if info.is_audio_opus {
                        "→ copy"
                    } else {
                        "→ Opus"
                    };
                    let info_text = format!(
                        "Video: {} ({}) | Audio: {} ({}) | Duración: {:.1}s",
                        info.video_codec,
                        video_hint,
                        info.audio_codec,
                        audio_hint,
                        info.duration_secs
                    );

                    window.set_info_text(info_text.into());
                    window.set_info_visible(true);
                    window.set_output_enabled(true);
                    window.set_convert_enabled(true);
                    window.set_status_text("Preparado".into());

                    s.input_path = Some(path);
                    s.output_path = Some(auto_out);
                    s.input_info = Some(info);
                }
                Err(e) => {
                    window.set_status_text(format!("Error: {e}").into());
                    window.set_convert_enabled(false);
                }
            }
        });
    }

    {
        let state = state.clone();
        let window_weak = window.as_weak();
        window.on_pick_output(move || {
            let Some(window) = window_weak.upgrade() else {
                return;
            };
            let ext = converter::OUTPUT_EXTENSION;
            let dialog = rfd::FileDialog::new()
                .add_filter(format!(".{ext}"), &[ext])
                .set_title("Guardar archivo de salida");

            let dialog = if let Some(ref path) = state.borrow().output_path {
                dialog.set_file_name(&*path.file_name().unwrap_or_default().to_string_lossy())
            } else {
                dialog
            };

            if let Some(path) = dialog.save_file() {
                window.set_output_path(path.display().to_string().into());
                state.borrow_mut().output_path = Some(path);
            }
        });
    }

    {
        let state = state.clone();
        let window_weak = window.as_weak();
        window.on_mode_changed(move |index| {
            let Some(window) = window_weak.upgrade() else {
                return;
            };
            let mode = if index == 1 {
                EncodeMode::CBR
            } else {
                EncodeMode::CRF
            };
            state.borrow_mut().encode_mode = mode;
            window.set_bitrate_visible(index == 1);
        });
    }

    {
        let state = state.clone();
        let progress_state = progress_state.clone();
        let window_weak = window.as_weak();
        window.on_convert_clicked(move || {
            let Some(window) = window_weak.upgrade() else {
                return;
            };

            let crf_value = parse_crf(window.get_crf_text().to_string().as_str());
            let bitrate_kbps = parse_bitrate(window.get_bitrate_text().to_string().as_str());

            let (input, output, input_info, config) = {
                let s = state.borrow();
                let input = s.input_path.clone();
                let output = s.output_path.clone();
                let input_info = s.input_info.clone();
                let config = EncodeConfig {
                    mode: s.encode_mode,
                    crf_value,
                    bitrate_kbps,
                };
                match (input, output, input_info) {
                    (Some(i), Some(o), Some(info)) => (i, o, info, config),
                    _ => return,
                }
            };

            window.set_convert_enabled(false);
            window.set_convert_label("Convirtiendo…".into());
            window.set_status_text("Iniciando conversión...".into());

            let ps = progress_state.clone();
            let window_weak2 = window.as_weak();
            std::thread::spawn(move || {
                let ps_for_progress = ps.clone();
                let result =
                    converter::convert(input, output, config, &input_info, |progress, _time| {
                        ps_for_progress
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .progress = progress;
                    });

                {
                    let mut ps = ps.lock().unwrap_or_else(|e| e.into_inner());
                    match result {
                        Ok(()) => {
                            ps.done = true;
                        }
                        Err(e) => {
                            ps.error = Some(e);
                        }
                    }
                }

                let _ = slint::invoke_from_event_loop(move || {
                    let Some(window) = window_weak2.upgrade() else {
                        return;
                    };
                    let mut ps = ps.lock().unwrap_or_else(|e| e.into_inner());

                    if let Some(err) = ps.error.take() {
                        window.set_status_text(format!("Error: {err}").into());
                        window.set_convert_enabled(false);
                        window.set_convert_label("Convertir".into());
                    }

                    if ps.done {
                        ps.done = false;
                        window.set_progress_value(0.0);
                        window.set_status_text("Preparado".into());
                        window.set_convert_enabled(true);
                        window.set_convert_label("Convertir".into());
                    }
                });
            });
        });
    }

    let progress_timer = slint::Timer::default();
    {
        let window_weak = window.as_weak();
        let ps = progress_state.clone();
        progress_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(500),
            move || {
                let Some(window) = window_weak.upgrade() else {
                    return;
                };
                let mut ps = ps.lock().unwrap_or_else(|e| e.into_inner());

                if ps.progress > 0.0 {
                    window.set_progress_value(ps.progress as f32);
                    ps.progress = 0.0;
                }

                if !ps.status.is_empty() {
                    window.set_status_text(ps.status.as_str().into());
                    ps.status.clear();
                }
            },
        );
    }

    window.run().unwrap();
}
