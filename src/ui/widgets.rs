use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::gtk::glib::clone;
use libadwaita::prelude::*;

use crate::app::{AppState, ProgressState};
use crate::converter::{self, Container, EncodeConfig, EncodeMode};

use super::dialogs;

pub fn setup_container_row(
    container_row: &libadwaita::ComboRow,
    state: &Rc<RefCell<AppState>>,
    output_row: &libadwaita::ActionRow,
    container: &Rc<Cell<Container>>,
) {
    let cont_model = libadwaita::gtk::StringList::new(&["MKV", "MP4"]);
    container_row.set_model(Some(&cont_model));

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
}

pub fn setup_mode_combo(
    mode_combo: &libadwaita::ComboRow,
    encode_mode: &Rc<Cell<EncodeMode>>,
    crf_row: &libadwaita::EntryRow,
    bitrate_row: &libadwaita::EntryRow,
) {
    let mode_model = libadwaita::gtk::StringList::new(&[
        "CRF (calidad constante)",
        "Bitrate constante (CBR)",
    ]);
    mode_combo.set_model(Some(&mode_model));
    mode_combo.set_property("selected", 0u32);

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
}

pub fn setup_crf_row(crf_row: &libadwaita::EntryRow, crf_value: &Rc<Cell<i32>>) {
    crf_row.connect_activate(clone!(
        #[strong]
        crf_value,
        move |entry| {
            if let Ok(v) = entry.text().to_string().parse::<i32>() {
                crf_value.set(v.clamp(0, 63));
            }
        }
    ));
}

pub fn setup_bitrate_row(
    bitrate_row: &libadwaita::EntryRow,
    bitrate_kbps: &Rc<Cell<i32>>,
) {
    bitrate_row.connect_activate(clone!(
        #[strong]
        bitrate_kbps,
        move |entry| {
            if let Ok(v) = entry.text().to_string().parse::<i32>() {
                bitrate_kbps.set(v.max(100));
            }
        }
    ));
}

pub fn setup_pick_input_btn(
    pick_input_btn: &libadwaita::gtk::Button,
    window: &libadwaita::ApplicationWindow,
    state: &Rc<RefCell<AppState>>,
    input_path_label: &libadwaita::gtk::Label,
    info_label: &libadwaita::gtk::Label,
    info_revealer: &libadwaita::gtk::Revealer,
    output_row: &libadwaita::ActionRow,
    pick_output_btn: &libadwaita::gtk::Button,
    container: Container,
    convert_button: &libadwaita::gtk::Button,
    status_label: &libadwaita::gtk::Label,
) {
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
        convert_button,
        #[strong]
        status_label,
        move |_| {
            dialogs::open_input_dialog(
                &window,
                &state,
                &input_path_label,
                &info_label,
                &info_revealer,
                &output_row,
                &pick_output_btn,
                container,
                &convert_button,
                &status_label,
            );
        }
    ));
}

pub fn setup_pick_output_btn(
    pick_output_btn: &libadwaita::gtk::Button,
    window: &libadwaita::ApplicationWindow,
    state: &Rc<RefCell<AppState>>,
    output_row: &libadwaita::ActionRow,
    container: Container,
) {
    pick_output_btn.connect_clicked(clone!(
        #[strong]
        state,
        #[strong]
        window,
        #[strong]
        output_row,
        move |_| {
            dialogs::open_output_dialog(&window, &state, &output_row, container);
        }
    ));
}

pub fn setup_convert_button(
    convert_button: &libadwaita::gtk::Button,
    state: &Rc<RefCell<AppState>>,
    encode_mode: &Rc<Cell<EncodeMode>>,
    crf_value: &Rc<Cell<i32>>,
    bitrate_kbps: &Rc<Cell<i32>>,
    status_label: &libadwaita::gtk::Label,
    progress_state: &Arc<Mutex<ProgressState>>,
) {
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
}

pub fn setup_progress_timer(
    progress_state: &Arc<Mutex<ProgressState>>,
    convert_button: &libadwaita::gtk::Button,
    status_label: &libadwaita::gtk::Label,
    progress_bar: &libadwaita::gtk::ProgressBar,
) {
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
}
