use std::cell::RefCell;
use std::rc::Rc;

use libadwaita::gtk::gio;
use libadwaita::gtk::glib::clone;
use libadwaita::prelude::*;

use resolvefy_core::AppState;
use resolvefy_core::converter;

pub struct InputWidgets<'a> {
    pub state: &'a Rc<RefCell<AppState>>,
    pub input_path_label: &'a libadwaita::gtk::Label,
    pub info_label: &'a libadwaita::gtk::Label,
    pub info_revealer: &'a libadwaita::gtk::Revealer,
    pub output_row: &'a libadwaita::ActionRow,
    pub pick_output_btn: &'a libadwaita::gtk::Button,
    pub convert_button: &'a libadwaita::gtk::Button,
    pub status_label: &'a libadwaita::gtk::Label,
}

pub fn create_file_filter_video() -> libadwaita::gtk::FileFilter {
    let filter = libadwaita::gtk::FileFilter::new();
    filter.set_name(Some("Vídeo"));
    ["mp4", "mkv", "avi", "mov", "webm", "flv", "ts"]
        .iter()
        .for_each(|ext| filter.add_suffix(ext));
    filter
}

pub fn create_file_filter_output() -> libadwaita::gtk::FileFilter {
    let filter = libadwaita::gtk::FileFilter::new();
    filter.set_name(Some(&format!("*.{}", converter::OUTPUT_EXTENSION)));
    filter.add_suffix(converter::OUTPUT_EXTENSION);
    filter
}

fn handle_input_file_selection(
    file: &gio::File,
    w: &InputWidgets<'_>,
) {
    let path = match file.path() {
        Some(path) => path,
        None => return,
    };

    w.input_path_label.set_label(&path.display().to_string());
    w.input_path_label.remove_css_class("dim-label");

    match converter::detect_input(&path) {
        Ok(info) => {
            let auto_out = converter::default_output_name(&path);
            w.output_row.set_subtitle(&auto_out.display().to_string());

            let video_hint = if info.is_video_av1 { "→ copy" } else { "→ SVT-AV1" };
            let audio_hint = if info.is_audio_opus { "→ copy" } else { "→ Opus" };
            let info_text = format!(
                "Video: {} ({}) | Audio: {} ({}) | Duración: {:.1}s",
                info.video_codec, video_hint, info.audio_codec, audio_hint, info.duration_secs
            );

            w.info_label.set_label(&info_text);
            w.info_revealer.set_reveal_child(true);

            let mut state_mut = w.state.borrow_mut();
            state_mut.input_path = Some(path);
            state_mut.output_path = Some(auto_out);
            state_mut.input_info = Some(info);

            w.pick_output_btn.set_sensitive(true);
            w.convert_button.set_sensitive(true);
        }
        Err(e) => {
            w.status_label.set_label(&format!("Error: {e}"));
            w.convert_button.set_sensitive(false);
        }
    }
}

fn handle_output_file_selection(
    file: &gio::File,
    state: &Rc<RefCell<AppState>>,
    output_row: &libadwaita::ActionRow,
) {
    let path = match file.path() {
        Some(path) => path,
        None => return,
    };

    output_row.set_subtitle(&path.display().to_string());
    state.borrow_mut().output_path = Some(path);
}

pub fn open_input_dialog(
    window: &libadwaita::ApplicationWindow,
    w: &InputWidgets<'_>,
) {
    let video_filter = create_file_filter_video();
    let filters = gio::ListStore::new::<libadwaita::gtk::FileFilter>();
    filters.append(&video_filter);

    let dialog = libadwaita::gtk::FileDialog::builder()
        .title("Seleccionar vídeo")
        .modal(true)
        .build();

    dialog.set_filters(Some(&filters));
    dialog.set_default_filter(Some(&video_filter));

    let state = w.state;
    let input_path_label = w.input_path_label;
    let info_label = w.info_label;
    let info_revealer = w.info_revealer;
    let output_row = w.output_row;
    let pick_output_btn = w.pick_output_btn;
    let convert_button = w.convert_button;
    let status_label = w.status_label;

    dialog.open(
        Some(window),
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
            convert_button,
            #[strong]
            status_label,
            move |result| {
                if let Ok(file) = result {
                    let w = InputWidgets {
                        state: &state,
                        input_path_label: &input_path_label,
                        info_label: &info_label,
                        info_revealer: &info_revealer,
                        output_row: &output_row,
                        pick_output_btn: &pick_output_btn,
                        convert_button: &convert_button,
                        status_label: &status_label,
                    };
                    handle_input_file_selection(&file, &w);
                }
            }
        ),
    );
}

pub fn open_output_dialog(
    window: &libadwaita::ApplicationWindow,
    state: &Rc<RefCell<AppState>>,
    output_row: &libadwaita::ActionRow,
) {
    let output_filter = create_file_filter_output();
    let filters = gio::ListStore::new::<libadwaita::gtk::FileFilter>();
    filters.append(&output_filter);

    let dialog = libadwaita::gtk::FileDialog::builder()
        .title("Guardar archivo de salida")
        .accept_label("Guardar")
        .modal(true)
        .build();

    dialog.set_filters(Some(&filters));
    dialog.set_default_filter(Some(&output_filter));

    if let Some(path) = state.borrow().output_path.as_ref() {
        dialog.set_initial_file(Some(&gio::File::for_path(path)));
    }

    dialog.save(
        Some(window),
        None::<&gio::Cancellable>,
        clone!(
            #[strong]
            state,
            #[strong]
            output_row,
            move |result| {
                if let Ok(file) = result {
                    handle_output_file_selection(&file, &state, &output_row);
                }
            }
        ),
    );
}
