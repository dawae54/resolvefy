use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::gtk::gio;
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

        // Grab toolbar and headerbar and ensure header is set as top bar
        let toolbar_view: libadwaita::ToolbarView =
            builder.object("toolbar_view").expect("toolbar_view");
        let header_bar: libadwaita::HeaderBar = builder.object("header_bar").expect("header_bar");
        let clamp: libadwaita::Clamp = builder.object("clamp").expect("clamp");

        // Ensure toolbar has the header as top bar and the clamp as content
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&clamp));

        // Widgets (IDs come from ui/app.blp)
        let pick_input_btn: libadwaita::gtk::Button =
            builder.object("pick_input_btn").expect("pick_input_btn");
        let input_path_label: libadwaita::gtk::Label = builder
            .object("input_path_label")
            .expect("input_path_label");
        let info_label: libadwaita::gtk::Label = builder.object("info_label").expect("info_label");
        let info_revealer: libadwaita::gtk::Revealer =
            builder.object("info_revealer").expect("info_revealer");

        let pick_output_btn: libadwaita::gtk::Button =
            builder.object("pick_output_btn").expect("pick_output_btn");
        let output_row: libadwaita::ActionRow = builder.object("output_row").expect("output_row");
        let container_row: libadwaita::ComboRow =
            builder.object("container_row").expect("container_row");

        let mode_combo: libadwaita::ComboRow = builder.object("mode_combo").expect("mode_combo");
        let crf_row: libadwaita::EntryRow = builder.object("crf_row").expect("crf_row");
        let bitrate_row: libadwaita::EntryRow = builder.object("bitrate_row").expect("bitrate_row");

        let convert_button: libadwaita::gtk::Button =
            builder.object("convert_button").expect("convert_button");
        let status_label: libadwaita::gtk::Label =
            builder.object("status_label").expect("status_label");
        let progress_bar: libadwaita::gtk::ProgressBar =
            builder.object("progress_bar").expect("progress_bar");

        // Set models that were previously set programmatically
        let cont_model = libadwaita::gtk::StringList::new(&["MKV", "MP4"]);
        container_row.set_model(Some(&cont_model));

        let mode_model = libadwaita::gtk::StringList::new(&[
            "CRF (calidad constante)",
            "Bitrate constante (CBR)",
        ]);
        mode_combo.set_model(Some(&mode_model));
        mode_combo.set_property("selected", 0u32);

        // Ensure window is attached to the application
        window.set_application(Some(app));
        window.present();

        // Hook signals (logic mostly unchanged, now using widgets from the builder)
        pick_input_btn.connect_clicked(clone!(
            #[strong]
            state,
            #[strong]
            window,
            #[strong]
            input_path_label,
            #[strong]
            info_label,
            #[strong]
            info_revealer,
            #[strong]
            output_row,
            #[strong]
            pick_output_btn,
            #[strong]
            container,
            #[strong]
            convert_button,
            #[strong]
            status_label,
            move |_| {
                let dialog = libadwaita::gtk::FileDialog::builder()
                    .title("Seleccionar vídeo")
                    .modal(true)
                    .build();

                let video_filter = libadwaita::gtk::FileFilter::new();
                video_filter.set_name(Some("Vídeo"));
                for ext in ["mp4", "mkv", "avi", "mov", "webm", "flv", "ts"] {
                    video_filter.add_suffix(ext);
                }

                let filters = gio::ListStore::new::<libadwaita::gtk::FileFilter>();
                filters.append(&video_filter);
                dialog.set_filters(Some(&filters));
                dialog.set_default_filter(Some(&video_filter));

                dialog.open(
                    Some(&window),
                    None::<&gio::Cancellable>,
                    clone!(
                        #[strong]
                        state,
                        #[strong]
                        input_path_label,
                        #[strong]
                        info_label,
                        #[strong]
                        info_revealer,
                        #[strong]
                        output_row,
                        #[strong]
                        pick_output_btn,
                        #[strong]
                        container,
                        #[strong]
                        convert_button,
                        #[strong]
                        status_label,
                        move |result| {
                            let file = match result {
                                Ok(file) => file,
                                Err(_) => return,
                            };

                            let path = match file.path() {
                                Some(path) => path,
                                None => return,
                            };

                            let path_str = path.display().to_string();
                            input_path_label.set_label(&path_str);
                            input_path_label.remove_css_class("dim-label");

                            match converter::detect_input(&path) {
                                Ok(info) => {
                                    let c = container.get();
                                    let auto_out = converter::default_output_name(&path, c);
                                    let out_str = auto_out.display().to_string();
                                    output_row.set_subtitle(&out_str);

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
                    ),
                );
            }
        ));

        pick_output_btn.connect_clicked(clone!(
            #[strong]
            state,
            #[strong]
            window,
            #[strong]
            output_row,
            #[strong]
            container,
            move |_| {
                let c = container.get();
                let ext = converter::output_extension(c);
                let filter_name = format!("*.{ext}");

                let dialog = libadwaita::gtk::FileDialog::builder()
                    .title("Guardar archivo de salida")
                    .accept_label("Guardar")
                    .modal(true)
                    .build();

                let output_filter = libadwaita::gtk::FileFilter::new();
                output_filter.set_name(Some(&filter_name));
                output_filter.add_suffix(ext);

                let filters = gio::ListStore::new::<libadwaita::gtk::FileFilter>();
                filters.append(&output_filter);
                dialog.set_filters(Some(&filters));
                dialog.set_default_filter(Some(&output_filter));

                if let Some(ref path) = state.borrow().output_path {
                    let initial_file = gio::File::for_path(path);
                    dialog.set_initial_file(Some(&initial_file));
                }

                dialog.save(
                    Some(&window),
                    None::<&gio::Cancellable>,
                    clone!(
                        #[strong]
                        state,
                        #[strong]
                        output_row,
                        move |result| {
                            let file = match result {
                                Ok(file) => file,
                                Err(_) => return,
                            };

                            let path = match file.path() {
                                Some(path) => path,
                                None => return,
                            };

                            let path_str = path.display().to_string();
                            output_row.set_subtitle(&path_str);
                            state.borrow_mut().output_path = Some(path);
                        }
                    ),
                );
            }
        ));

        container_row.connect_selected_notify(clone!(
            #[strong]
            state,
            #[strong]
            output_row,
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
                    let auto_out = converter::default_output_name(path, c);
                    let out_str = auto_out.display().to_string();
                    output_row.set_subtitle(&out_str);
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

        mode_combo.connect_notify_local(
            Some("selected"),
            clone!(
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
            ),
        );

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
                            // progress is 0-100 from converter; store fraction 0.0-1.0
                            ps.progress = (progress / 100.0) as f32;
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
                        let frac = ps.progress as f64;
                        progress_bar.set_fraction(frac);
                        let percent = (frac * 100.0).round() as u8;
                        status_label.set_label(&format!("{}%", percent));
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
                        status_label.set_label("Preparado");
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
