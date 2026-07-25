use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::gtk::glib::clone;
use libadwaita::prelude::*;
use libadwaita::gtk::{Box as GtkBox, Orientation};

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

        let toolbar_view = libadwaita::ToolbarView::new();
        toolbar_view.add_top_bar(&libadwaita::HeaderBar::new());

        let clamp = libadwaita::Clamp::builder()
            .maximum_size(600)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .build();

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.set_spacing(12);

        let title = libadwaita::gtk::Label::builder()
            .label("Resolvefy")
            .css_classes(["title-1"])
            .halign(libadwaita::gtk::Align::Center)
            .build();
        main_box.append(&title);

        let subtitle = libadwaita::gtk::Label::builder()
            .label("Conversor de vídeo a AV1")
            .css_classes(["dim-label"])
            .halign(libadwaita::gtk::Align::Center)
            .build();
        main_box.append(&subtitle);

        let input_group = libadwaita::PreferencesGroup::builder()
            .title("Entrada")
            .build();

        let pick_input_row = libadwaita::ActionRow::builder()
            .title("Archivo de vídeo")
            .activatable(true)
            .build();
        let pick_input_btn = libadwaita::gtk::Button::builder()
            .label("Seleccionar…")
            .valign(libadwaita::gtk::Align::Center)
            .build();
        pick_input_row.add_suffix(&pick_input_btn);
        pick_input_row.set_activatable_widget(Some(&pick_input_btn));
        input_group.add(&pick_input_row);

        let input_path_label = libadwaita::gtk::Label::builder()
            .label("Ningún archivo seleccionado")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build();
        input_group.add(&input_path_label);

        let info_label = libadwaita::gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .build();
        let info_revealer = libadwaita::gtk::Revealer::builder()
            .transition_type(libadwaita::gtk::RevealerTransitionType::SlideDown)
            .build();
        info_revealer.set_child(Some(&info_label));
        input_group.add(&info_revealer);
        main_box.append(&input_group);

        let output_group = libadwaita::PreferencesGroup::builder()
            .title("Salida")
            .build();

        let pick_output_row = libadwaita::ActionRow::builder()
            .title("Guardar como")
            .activatable(true)
            .build();
        let pick_output_btn = libadwaita::gtk::Button::builder()
            .label("Seleccionar…")
            .valign(libadwaita::gtk::Align::Center)
            .build();
        pick_output_btn.set_sensitive(false);
        pick_output_row.add_suffix(&pick_output_btn);
        pick_output_row.set_activatable_widget(Some(&pick_output_btn));
        output_group.add(&pick_output_row);

        let output_path_label = libadwaita::gtk::Label::builder()
            .label("Sin ruta de salida")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build();
        output_group.add(&output_path_label);

        let container_row = libadwaita::ComboRow::builder()
            .title("Contenedor")
            .build();
        let model = libadwaita::gtk::StringList::new(&["MKV", "MP4"]);
        container_row.set_model(Some(&model));
        output_group.add(&container_row);
        main_box.append(&output_group);

        let enc_group = libadwaita::PreferencesGroup::builder()
            .title("Codificación")
            .build();

        let mode_model = libadwaita::gtk::StringList::new(&[
            "CRF (calidad constante)",
            "Bitrate constante (CBR)",
        ]);

        let mode_combo = libadwaita::ComboRow::builder()
            .title("Modo de codificación")
            .model(&mode_model)
            .selected(0)
            .build();

        let crf_row = libadwaita::EntryRow::builder()
            .title("Valor CRF (0–63)")
            .text("30")
            .build();

        let bitrate_row = libadwaita::EntryRow::builder()
            .title("Bitrate (kbps)")
            .text("5000")
            .build();
        bitrate_row.set_visible(false);

        enc_group.add(&mode_combo);
        enc_group.add(&crf_row);
        enc_group.add(&bitrate_row);
        main_box.append(&enc_group);

        let convert_button = libadwaita::gtk::Button::builder()
            .label("Convertir")
            .css_classes(["suggested-action"])
            .halign(libadwaita::gtk::Align::End)
            .build();
        convert_button.set_sensitive(false);
        main_box.append(&convert_button);

        let status_label = libadwaita::gtk::Label::builder()
            .label("Preparado")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dim-label"])
            .build();
        main_box.append(&status_label);

        let progress_bar = libadwaita::gtk::ProgressBar::builder()
            .build();
        main_box.append(&progress_bar);

        clamp.set_child(Some(&main_box));
        toolbar_view.set_content(Some(&clamp));

        let window = libadwaita::ApplicationWindow::builder()
            .application(app)
            .title("Resolvefy")
            .default_width(650)
            .default_height(650)
            .content(&toolbar_view)
            .build();

        window.present();

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
