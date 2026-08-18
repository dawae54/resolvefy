use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita::prelude::*;

use resolvefy_core::converter::{self, EncodeConfig, EncodeMode};
use resolvefy_core::{AppState, ProgressState};

const UI_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/ui/window.ui");

fn video_filters() -> gio::ListStore {
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("Vídeo"));
    for ext in ["mp4", "mkv", "avi", "mov", "webm", "flv", "ts"] {
        filter.add_pattern(&format!("*.{ext}"));
    }
    let filters = gio::ListStore::new::<gtk4::FileFilter>();
    filters.append(&filter);
    filters
}

pub fn build_ui(app: &libadwaita::Application) {
    let builder = gtk4::Builder::from_file(UI_PATH);

    let window = builder
        .object::<libadwaita::Window>("window")
        .expect("falta 'window' en window.ui");
    window.set_application(Some(app));
    window.present();

    let input_row: libadwaita::ActionRow = builder.object("input_row").expect("id faltante");
    let input_button: gtk4::Button = builder.object("input_button").expect("id faltante");
    let info_label: gtk4::Label = builder.object("info_label").expect("id faltante");
    let output_row: libadwaita::ActionRow = builder.object("output_row").expect("id faltante");
    let output_button: gtk4::Button = builder.object("output_button").expect("id faltante");
    let mode_combo: gtk4::DropDown = builder.object("mode_combo").expect("id faltante");
    let crf_spin: gtk4::SpinButton = builder.object("crf_spin").expect("id faltante");
    let crf_row: libadwaita::ActionRow = builder.object("crf_row").expect("id faltante");
    let bitrate_spin: gtk4::SpinButton = builder.object("bitrate_spin").expect("id faltante");
    let bitrate_row: libadwaita::ActionRow = builder.object("bitrate_row").expect("id faltante");
    let status_label: gtk4::Label = builder.object("status_label").expect("id faltante");
    let percent_label: gtk4::Label = builder.object("percent_label").expect("id faltante");
    let progress_bar: gtk4::ProgressBar = builder.object("progress_bar").expect("id faltante");
    let convert_button: gtk4::Button = builder.object("convert_button").expect("id faltante");

    let state = Rc::new(RefCell::new(AppState::default()));
    let progress_state = Arc::new(Mutex::new(ProgressState::default()));

    let model = gtk4::StringList::new(&["CRF (calidad constante)", "CBR (bitrate constante)"]);
    mode_combo.set_model(Some(&model));

    mode_combo.connect_selected_notify(glib::clone!(
        #[strong]
        state,
        #[strong]
        crf_row,
        #[strong]
        bitrate_row,
        move |combo| {
            let is_cbr = combo.selected() == 1;
            state.borrow_mut().encode_mode = if is_cbr {
                EncodeMode::CBR
            } else {
                EncodeMode::CRF
            };
            crf_row.set_visible(!is_cbr);
            bitrate_row.set_visible(is_cbr);
        },
    ));

    let input_dialog = gtk4::FileDialog::builder()
        .title("Seleccionar vídeo")
        .filters(&video_filters())
        .build();
    input_button.connect_clicked(glib::clone!(
        #[strong]
        window,
        #[strong]
        input_dialog,
        #[strong]
        input_row,
        #[strong]
        info_label,
        #[strong]
        output_row,
        #[strong]
        output_button,
        #[strong]
        convert_button,
        #[strong]
        status_label,
        #[strong]
        state,
        move |_| {
            input_dialog.open(
                Some(&window),
                None::<&gio::Cancellable>,
                glib::clone!(
                    #[strong]
                    input_row,
                    #[strong]
                    info_label,
                    #[strong]
                    output_row,
                    #[strong]
                    output_button,
                    #[strong]
                    convert_button,
                    #[strong]
                    status_label,
                    #[strong]
                    state,
                    move |result| {
                        let Ok(file) = result else {
                            return;
                        };
                        let Some(path) = file.path() else {
                            return;
                        };

                        input_row.set_subtitle(&path.display().to_string());

                        match converter::detect_input(&path) {
                            Ok(info) => {
                                let auto_out = converter::default_output_name(&path);
                                output_row.set_subtitle(&auto_out.display().to_string());

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
                                info_label.set_label(&format!(
                                    "Video: {} ({}) | Audio: {} ({}) | Duración: {}",
                                    info.video_codec,
                                    video_hint,
                                    info.audio_codec,
                                    audio_hint,
                                    converter::format_duration(info.duration_secs)
                                ));
                                info_label.set_visible(true);
                                output_button.set_sensitive(true);
                                convert_button.set_sensitive(true);
                                status_label.set_label("Preparado");

                                let mut s = state.borrow_mut();
                                s.input_path = Some(path);
                                s.output_path = Some(auto_out);
                                s.input_info = Some(info);
                            }
                            Err(e) => {
                                status_label.set_label(&format!("Error: {e}"));
                                convert_button.set_sensitive(false);
                            }
                        }
                    },
                ),
            );
        },
    ));

    let output_dialog = gtk4::FileDialog::builder()
        .title("Guardar archivo de salida")
        .filters(&video_filters());
    let output_dialog = match state.borrow().output_path.clone() {
        Some(path) => output_dialog.initial_file(&gio::File::for_path(&path)),
        None => output_dialog,
    };
    let output_dialog = output_dialog.build();
    output_button.connect_clicked(glib::clone!(
        #[strong]
        window,
        #[strong]
        output_dialog,
        #[strong]
        output_row,
        #[strong]
        state,
        move |_| {
            output_dialog.save(
                Some(&window),
                None::<&gio::Cancellable>,
                glib::clone!(
                    #[strong]
                    output_row,
                    #[strong]
                    state,
                    move |result| {
                        let Ok(file) = result else {
                            return;
                        };
                        let Some(path) = file.path() else {
                            return;
                        };
                        output_row.set_subtitle(&path.display().to_string());
                        state.borrow_mut().output_path = Some(path);
                    },
                ),
            );
        },
    ));

    convert_button.connect_clicked(glib::clone!(
        #[strong]
        crf_spin,
        #[strong]
        bitrate_spin,
        #[strong]
        status_label,
        #[strong]
        convert_button,
        #[strong]
        state,
        #[strong]
        progress_state,
        move |_| {
            let crf_value = crf_spin.value() as u32;
            let bitrate_kbps = bitrate_spin.value() as u32;

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

            convert_button.set_sensitive(false);
            convert_button.set_label("Convirtiendo…");
            status_label.set_label("Iniciando conversión...");

            let ps = progress_state.clone();
            std::thread::spawn(move || {
                let ps_for_progress = ps.clone();
                let result = converter::convert(
                    input,
                    output,
                    config,
                    &input_info,
                    move |progress, _time| {
                        ps_for_progress
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .progress = progress;
                    },
                );

                let mut ps = ps.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => ps.done = true,
                    Err(e) => ps.error = Some(e),
                }
            });
        },
    ));

    glib::timeout_add_local(
        std::time::Duration::from_millis(200),
        glib::clone!(
            #[strong]
            status_label,
            #[strong]
            percent_label,
            #[strong]
            progress_bar,
            #[strong]
            convert_button,
            #[strong]
            progress_state,
            move || {
                let mut ps = progress_state.lock().unwrap_or_else(|e| e.into_inner());

                if ps.progress > 0.0 {
                    progress_bar.set_fraction(ps.progress / 100.0);
                    percent_label.set_label(&format!("{}%", ps.progress.round() as u32));
                    percent_label.set_visible(true);
                    ps.progress = 0.0;
                }

                if !ps.status.is_empty() {
                    status_label.set_label(&ps.status);
                    ps.status.clear();
                }

                if let Some(err) = ps.error.take() {
                    status_label.set_label(&format!("Error: {err}"));
                    convert_button.set_sensitive(true);
                    convert_button.set_label("Convertir");
                }

                if ps.done {
                    ps.done = false;
                    progress_bar.set_fraction(0.0);
                    percent_label.set_visible(false);
                    status_label.set_label("Preparado");
                    convert_button.set_sensitive(true);
                    convert_button.set_label("Convertir");
                }

                glib::ControlFlow::Continue
            },
        ),
    );
}