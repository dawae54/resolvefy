use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::gtk::glib::clone;
use libadwaita::prelude::*;

use crate::app::{AppState, ProgressState};
use crate::converter::{self, Container, EncodeConfig, EncodeMode};

use super::dialogs;

fn update_output_path(state: &Rc<RefCell<AppState>>, output_row: &libadwaita::ActionRow, container: Container) {
    state.borrow().input_path.as_ref().map(|path| {
        let auto_out = converter::default_output_name(path, container);
        output_row.set_subtitle(&auto_out.display().to_string());
        state.borrow_mut().output_path = Some(auto_out);
    });
}

pub fn setup_container_row(
    container_row: &libadwaita::ComboRow,
    state: &Rc<RefCell<AppState>>,
    output_row: &libadwaita::ActionRow,
    container: &Rc<Cell<Container>>,
) {
    container_row.set_model(Some(&libadwaita::gtk::StringList::new(&["MKV", "MP4"])));

    container_row.connect_selected_notify(clone!(
        #[strong]
        state,
        #[strong]
        output_row,
        #[strong]
        container,
        move |row| {
            matches!(row.selected(), 0 | 1).then(|| {
                let c = if row.selected() == 0 { Container::MKV } else { Container::MP4 };
                container.set(c);
                update_output_path(&state, &output_row, c);
            });
        }
    ));
}

pub fn setup_mode_combo(
    mode_combo: &libadwaita::ComboRow,
    encode_mode: &Rc<Cell<EncodeMode>>,
    crf_row: &libadwaita::EntryRow,
    bitrate_row: &libadwaita::EntryRow,
) {
    mode_combo.set_model(Some(&libadwaita::gtk::StringList::new(&[
        "CRF (calidad constante)",
        "Bitrate constante (CBR)",
    ])));
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
                let mode = if combo.property::<u32>("selected") == 0 {
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
            entry.text().to_string().parse::<i32>().ok().map(|v| crf_value.set(v.clamp(0, 63)));
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
            entry.text().to_string().parse::<i32>().ok().map(|v| bitrate_kbps.set(v.max(100)));
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
                &window, &state, &input_path_label, &info_label,
                &info_revealer, &output_row, &pick_output_btn,
                container, &convert_button, &status_label,
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

fn build_encode_config(encode_mode: &Rc<Cell<EncodeMode>>, crf_value: &Rc<Cell<i32>>, bitrate_kbps: &Rc<Cell<i32>>) -> EncodeConfig {
    EncodeConfig {
        mode: encode_mode.get(),
        crf_value: crf_value.get() as u32,
        bitrate_kbps: bitrate_kbps.get() as u32,
    }
}

fn spawn_conversion(
    input: std::path::PathBuf,
    output: std::path::PathBuf,
    config: EncodeConfig,
    input_info: crate::converter::InputInfo,
    progress_state: Arc<Mutex<ProgressState>>,
) {
    let ps = progress_state.clone();
    std::thread::spawn(move || {
        let ps_for_progress = ps.clone();
        let result = converter::convert(input, output, config, &input_info, |progress, _time| {
            ps_for_progress.lock().unwrap().progress = (progress / 100.0) as f32;
        });

        let mut ps = ps.lock().unwrap();
        match result {
            Ok(()) => { ps.done = true; }
            Err(e) => { ps.error = Some(e); }
        }
    });
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
            let (input, output, input_info) = {
                let state_ref = state.borrow();
                let input = state_ref.input_path.clone();
                let output = state_ref.output_path.clone();
                let input_info = state_ref.input_info.clone();
                match (input, output, input_info) {
                    (Some(i), Some(o), Some(info)) => (i, o, info),
                    _ => return,
                }
            };

            let config = build_encode_config(&encode_mode, &crf_value, &bitrate_kbps);

            convert_button.set_sensitive(false);
            convert_button.set_label("Convirtiendo…");
            status_label.set_label("Iniciando conversión...");

            spawn_conversion(input, output, config, input_info, progress_state.clone());
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
                    status_label.set_label(&format!("{}%", (frac * 100.0).round() as u8));
                    ps.progress = 0.0;
                }

                if !ps.status.is_empty() {
                    status_label.set_label(&ps.status);
                    ps.status.clear();
                }

                ps.error.take().map(|err| {
                    status_label.set_label(&format!("Error: {err}"));
                    convert_button.set_sensitive(false);
                    convert_button.set_label("Convertir");
                });

                if ps.done {
                    ps.done = false;
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
