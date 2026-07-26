pub mod dialogs;
pub mod widgets;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use libadwaita::prelude::*;

use crate::app::{AppState, ProgressState};
use crate::converter::{Container, EncodeMode};

const UI_XML: &str = include_str!(concat!(env!("OUT_DIR"), "/app.ui"));

pub fn build_ui(
    state: Rc<RefCell<AppState>>,
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

        let builder = libadwaita::gtk::Builder::from_string(UI_XML);

        let window: libadwaita::ApplicationWindow = builder
            .object("window")
            .expect("failed to get window from builder");

        let toolbar_view: libadwaita::ToolbarView =
            builder.object("toolbar_view").expect("toolbar_view");
        let header_bar: libadwaita::HeaderBar = builder.object("header_bar").expect("header_bar");
        let clamp: libadwaita::Clamp = builder.object("clamp").expect("clamp");

        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&clamp));

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

        window.set_application(Some(app));
        window.present();

        widgets::setup_container_row(&container_row, &state, &output_row, &container);
        widgets::setup_mode_combo(&mode_combo, &encode_mode, &crf_row, &bitrate_row);
        widgets::setup_crf_row(&crf_row, &crf_value);
        widgets::setup_bitrate_row(&bitrate_row, &bitrate_kbps);

        widgets::setup_pick_input_btn(
            &pick_input_btn,
            &window,
            &dialogs::InputWidgets {
                state: &state,
                input_path_label: &input_path_label,
                info_label: &info_label,
                info_revealer: &info_revealer,
                output_row: &output_row,
                pick_output_btn: &pick_output_btn,
                convert_button: &convert_button,
                status_label: &status_label,
            },
            container.get(),
        );

        widgets::setup_pick_output_btn(
            &pick_output_btn,
            &window,
            &state,
            &output_row,
            container.get(),
        );

        widgets::setup_convert_button(
            &convert_button,
            &state,
            &encode_mode,
            &crf_value,
            &bitrate_kbps,
            &status_label,
            &progress_state,
        );

        widgets::setup_progress_timer(
            &progress_state,
            &convert_button,
            &status_label,
            &progress_bar,
        );
    });

    app
}
