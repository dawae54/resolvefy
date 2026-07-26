use std::cell::RefCell;
use std::rc::Rc;

use libadwaita::gtk::gio;
use libadwaita::gtk::glib::clone;
use libadwaita::prelude::*;

use crate::app::AppState;
use crate::converter::{self, Container};

pub fn create_file_filter_video() -> libadwaita::gtk::FileFilter {
    let filter = libadwaita::gtk::FileFilter::new();
    filter.set_name(Some("Vídeo"));
    for ext in ["mp4", "mkv", "avi", "mov", "webm", "flv", "ts"] {
        filter.add_suffix(ext);
    }
    filter
}

pub fn create_file_filter_output(container: Container) -> libadwaita::gtk::FileFilter {
    let ext = converter::output_extension(container);

    let filter = libadwaita::gtk::FileFilter::new();
    filter.set_name(Some(&format!("*.{ext}")));
    filter.add_suffix(ext);
    filter
}

pub fn open_input_dialog(
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
    let dialog = libadwaita::gtk::FileDialog::builder()
        .title("Seleccionar vídeo")
        .modal(true)
        .build();

    let video_filter = create_file_filter_video();
    let filters = gio::ListStore::new::<libadwaita::gtk::FileFilter>();
    filters.append(&video_filter);
    dialog.set_filters(Some(&filters));
    dialog.set_default_filter(Some(&video_filter));

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
                        let auto_out = converter::default_output_name(&path, container);
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

pub fn open_output_dialog(
    window: &libadwaita::ApplicationWindow,
    state: &Rc<RefCell<AppState>>,
    output_row: &libadwaita::ActionRow,
    container: Container,
) {
    let dialog = libadwaita::gtk::FileDialog::builder()
        .title("Guardar archivo de salida")
        .accept_label("Guardar")
        .modal(true)
        .build();

    let output_filter = create_file_filter_output(container);
    let filters = gio::ListStore::new::<libadwaita::gtk::FileFilter>();
    filters.append(&output_filter);
    dialog.set_filters(Some(&filters));
    dialog.set_default_filter(Some(&output_filter));

    if let Some(ref path) = state.borrow().output_path {
        let initial_file = gio::File::for_path(path);
        dialog.set_initial_file(Some(&initial_file));
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
