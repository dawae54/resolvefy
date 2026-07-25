use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::gtk::glib::clone;
use libadwaita::prelude::*;

// Include the generated UI XML produced by build.rs into the binary.
const UI_XML: &str = include_str!(concat!(env!("OUT_DIR"), "/app.ui"));

use crate::converter::{self, Container, EncodeConfig, EncodeMode};
use crate::{ProgressState, State};

pub fn build_ui(
    state: Rc<RefCell<State>>,
    progress_state: Arc<Mutex<ProgressState>>,
) -> libadwaita::Application {
    let app = libadwaita::Application::builder()
        .application_id("com.github.resolvefy")
        .build();

    app.connect_activate(move |app| {
        let encode_mode = Rc::new(Cell::new(EncodeMode::CRF));
        let container = Rc::new(Cell::new(Container::MKV));
        let crf_value = Rc::new(Cell::new(30i32));
        let bitrate_kbps = Rc::new(Cell::new(5000i32));

        // Load UI from compiled blueprint XML
        let builder = libadwaita::gtk::Builder::from_string(UI_XML);

        let window: libadwaita::ApplicationWindow = builder
            .object("window")
            .expect("failed to get window from builder");

        // Widgets (IDs come from ui/app.blp, without the $ prefix)
        let pick_input_btn: libadwaita::gtk::Button = builder
            .object("pick_input_btn")
            .expect("pick_input_btn");
        let input_path_label: libadwaita::gtk::Label = builder
            .object("input_path_label")
            .expect("input_path_label");
        let info_label: libadwaita::gtk::Label = builder
            .object("info_label")
            .expect("info_label");
        let info_revealer: libadwaita::gtk::Revealer = builder
            .object("info_revealer")
            .expect("info_revealer");

        let pick_output_btn: libadwaita::gtk::Button = builder
            .object("pick_output_btn")
            .expect("pick_output_btn");
        let output_path_label: libadwaita::gtk::Label = builder
            .object("output_path_label")
            .expect("output_path_label");
        let container_row: libadwaita::ComboRow = builder
            .object("container_row")
            .expect("container_row");

        let mode_combo: libadwaita::ComboRow = builder
            .object("mode_combo")
            .expect("mode_combo");
        let crf_row: libadwaita::EntryRow = builder
            .object("crf_row")
            .expect("crf_row");
        let bitrate_row: libadwaita::EntryRow = builder
            .object("bitrate_row")
            .expect("bitrate_row");

        let convert_button: libadwaita::gtk::Button = builder
            .object("convert_button")
            .expect("convert_button");
        let status_label: libadwaita::gtk::Label = builder
            .object("status_label")
            .expect("status_label");
        let progress_bar: libadwaita::gtk::ProgressBar = builder
            .object("progress_bar")
            .expect("progress_bar");

        // Set models that were previously set programmatically
        let cont_model = libadwaita::gtk::StringList::new(&["MKV", "MP4"]);
        container_row.set_model(Some(&cont_model));

        let mode_model = libadwaita::gtk::StringList::new(&[
            "CRF (calidad constante)",
            "Bitrate constante (CBR)",
        ]);
        mode_combo.set_model(Some(&mode_model));
        mode_combo.set_property("selected", &0u32);

        // Ensure window is attached to the application
        window.set_application(Some(app));
        window.present();

        // Hook signals (logic mostly unchanged, now using widgets from the builder)
        pick_input_btn.connect_clicked(clone!(
            #[strong]
            state,
            #[strong]
            input_path_label,
            #[strong]
            info_label,
            #[strong]
            info_revealer,
            #[strong]
            output_path_label,
            #[strong]
            pick_output_btn,
            #[strong]
            container,
            #[strong]
            convert_button,
            #[strong]
            status_label,
            move |_| {
                let file = rfd::FileDialog::new()
                    .add_filter(
                        "Vídeo",
                        &["mp4", "mkv", "avi", "mov", "webm", "flv", "ts"],
                    )
                    .pick_file();

                if let Some(path) = file {
                    let path_str = path.display().to_string();
                    input_path_label.set_label(&path_str);
                    input_path_label.remove_css_class("dim-label");

                    match converter::detect_input(&path) {
                        Ok(info) => {
                            let c = container.get();
                            let auto_out =
                                converter::default_output_name(&path, c);
                            let out_str = auto_out.display().to_string();
                            output_path_label.set_label(&out_str);
                            output_path_label.remove_css_class("dim-label");

                            let video_hint =
                                if info.is_video_av1 { "→ copy" } else { "→ SVT-AV1" };
                            let audio_hint =
                                if info.is_audio_opus { "→ copy" } else { "→ Opus" };
                            let info_text = format!(
                                "Video: {} ({}) | Audio: {} ({}) | Duración: {:.1}s",
                                info.video_codec,
                                video_hint,
                                info.audio_codec,
                                audio_hint,
                                info.duration_secs
                            );
                            info_label.set_label(&info_text);
                            info_revealer.set_reveal_child(true);

                            state.borrow_mut().input_path = Some(path);
                            state.borrow_mut().output_path = Some(auto_out);
                            state.borrow_mut().input_info = Some(info);
                            pick_output_btn.set_sensitive(true);
                            convert_button.set_sensitive(true);
                        }
                        Err(e) => {
                            status_label.set_label(&format!("Error: {e}"));
                            convert_button.set_sensitive(false);
                        }
                    }
                }
            }
        ));

        pick_output_btn.connect_clicked(clone!(
            #[strong]
            state,
            #[strong]
            output_path_label,
            #[strong]
            container,
            move |_| {
                let c = container.get();
                let ext = converter::output_extension(c);
                let filter_name = format!("*.{ext}");

                let mut dialog =
                    rfd::FileDialog::new().add_filter(&filter_name, &[ext]);

                if let Some(ref path) = state.borrow().output_path {
                    if let Some(dir) = path.parent() {
                        dialog = dialog.set_directory(dir);
                    }
                }

                if let Some(path) = dialog.save_file() {
                    let path_str = path.display().to_string();
                    output_path_label.set_label(&path_str);
                    output_path_label.remove_css_class("dim-label");
                    state.borrow_mut().output_path = Some(path);
                }
            }
        ));

        container_row.connect_selected_notify(clone!(
            #[strong]
            state,
            #[strong]
            output_path_label,
            #[strong]
            container,
            move |row| {
                let c = match row.selected() {
                    0 => Container::MKV,
                    1 => Container::MP4,
                    _ => return,
                };
                container.set(c);

                let input_path = state.borrow().input_path.clone();
                if let Some(ref path) = input_path {
                    let auto_out =
                        converter::default_output_name(path, c);
                    let out_str = auto_out.display().to_string();
                    output_path_label.set_label(&out_str);
                    output_path_label.remove_css_class("dim-label");
                    state.borrow_mut().output_path = Some(auto_out);
                }
            }
        ));

        crf_row.connect_activate(clone!(
            #[strong]
            crf_value,
            move |entry| {
                if let Ok(v) = entry.text().to_string().parse::<i32>() {
                    crf_value.set(v.clamp(0, 63));
                }
            }
        ));

        bitrate_row.connect_activate(clone!(
            #[strong]
            bitrate_kbps,
            move |entry| {
                if let Ok(v) = entry.text().to_string().parse::<i32>() {
                    bitrate_kbps.set(v.max(100));
                }
            }
        ));

        mode_combo.connect_notify_local(Some("selected"), clone!(
            #[strong]
            encode_mode,
            #[strong]
            crf_row,
            #[strong]
            bitrate_row,
            move |combo, _pspec| {
                let selected = combo.property::<u32>("selected");
                let mode = if selected == 0 {
                    EncodeMode::CRF
                } else {
                    EncodeMode::CBR
                };
                encode_mode.set(mode);
                crf_row.set_visible(matches!(mode, EncodeMode::CRF));
                bitrate_row.set_visible(matches!(mode, EncodeMode::CBR));
            }
        ));

        convert_button.connect_clicked(clone!(
            #[strong]
            state,
            #[strong]
            encode_mode,
            #[strong]
            crf_value,
            #[strong]
            bitrate_kbps,
            #[strong]
            convert_button,
            #[strong]
            status_label,
            #[strong]
            progress_state,
            move |_| {
                let state_ref = state.borrow();

                let input = match state_ref.input_path {
                    Some(ref p) => p.clone(),
                    None => return,
                };
                let output = match state_ref.output_path {
                    Some(ref p) => p.clone(),
                    None => return,
                };
                let input_info = match state_ref.input_info {
                    Some(ref i) => i.clone(),
                    None => return,
                };

                let config = EncodeConfig {
                    mode: encode_mode.get(),
                    crf_value: crf_value.get() as u32,
                    bitrate_kbps: bitrate_kbps.get() as u32,
                };

                drop(state_ref);

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
                        |progress, _time| {
                            let mut ps = ps_for_progress.lock().unwrap();
                            ps.progress = (progress / 100.0) as f32;
                            if _time != "done" {
                                ps.status = format!("Progreso: {}%", progress as u8);
                            }
                        },
                    );

                    let mut ps = ps.lock().unwrap();
                    match result {
                        Ok(()) => {
                            ps.done = true;
                        }
                        Err(e) => {
                            ps.error = Some(e);
                        }
                    }
                });
            }
        ));

        let ps = progress_state.clone();
        libadwaita::gtk::glib::timeout_add_local(
            std::time::Duration::from_millis(100),
            clone!(
                #[strong]
                convert_button,
                #[strong]
                status_label,
                #[strong]
                progress_bar,
                move || {
                    let mut ps = ps.lock().unwrap();
                    if ps.progress > 0.0 {
                        progress_bar.set_fraction(ps.progress as f64);
                        ps.progress = 0.0;
                    }
                    if !ps.status.is_empty() {
                        status_label.set_label(&ps.status);
                        ps.status.clear();
                    }
                    if let Some(err) = ps.error.take() {
                        status_label.set_label(&format!("Error: {err}"));
                        convert_button.set_sensitive(false);
                        convert_button.set_label("Convertir");
                    }
                    if ps.done {
                        ps.done = false;
                        status_label.set_label("Conversión completada.");
                        progress_bar.set_fraction(0.0);
                        convert_button.set_sensitive(true);
                        convert_button.set_label("Convertir");
                    }
                    libadwaita::gtk::glib::ControlFlow::Continue
                }
            ),
        );
    });

    app
}
