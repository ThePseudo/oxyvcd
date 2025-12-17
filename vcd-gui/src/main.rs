use slint::SharedString;
use std::sync::Mutex;
use std::thread;
use vcd_statistical_analysis::{perform_analysis, Configuration};

slint::slint! {
    import { Button, GroupBox, LineEdit } from "std-widgets.slint";

    export component MainWindow inherits Window {
        in-out property<string> in_path <=> inpath.text;
        out property<string> out_path <=> outpath.text;
        out property<string> separator <=> sep.text;
        in property<bool> interface_enabled;
        in property<bool> button_enabled;
        in property<string> status_text <=> status_label.text;

        callback button-pressed <=> evaluate_button.clicked;
        callback browse-source-pressed <=> browse_source_button.clicked;
        callback in-file-edited <=> inpath.edited;
        callback out-file-edited <=> outpath.edited;

        preferred-width: 640px;
        preferred-height: 480px;
        min-width: 420px;
        min-height: 240px;
        title: "vcd";
        VerticalLayout {
            padding: 8px;
            alignment: center;
            spacing: 32px;
            GridLayout {
                spacing: 32px;
                Row {
                    Text {
                        vertical-alignment: center;
                        text: "Input VCD file path";

                    }
                    inpath := LineEdit {
                        placeholder-text: "Path";
                        read-only: !interface_enabled;
                    }
                    browse_source_button := Button {
                        text: "Browse";
                    }
                }
                Row {
                    Text {
                        vertical-alignment: center;
                        text: "Output file path";
                    }
                    outpath := LineEdit {
                        placeholder-text: "Path";
                        read-only: !interface_enabled;
                    }
                }
                Row {
                    Text {
                        vertical-alignment: center;
                        text: "Separator character (default: ' ')";
                    }
                    sep := LineEdit {
                        placeholder-text: "Separator";
                        read-only: !interface_enabled;
                    }
                }
            }
            evaluate_button := Button {
                text: "Perform analysis";
                enabled: interface_enabled && button_enabled;
                primary: true;
            }
            status_label := Text {
                vertical-alignment: center;
                text: "";
            }
        }
    }
}

fn main() {
    let window = MainWindow::new().unwrap();
    window.set_interface_enabled(true);
    window.set_button_enabled(false);
    {
        let weak_window_source_pressed = window.as_weak();
        window.on_browse_source_pressed(move || {
            let in_window = weak_window_source_pressed.upgrade().unwrap();
            let path = native_dialog::DialogBuilder::file()
                .set_location(&in_window.get_in_path().to_string())
                .add_filter("VCD file", ["vcd"])
                .open_single_file()
                .show()
                .unwrap();
            if path.is_some() {
                weak_window_source_pressed
                    .upgrade()
                    .unwrap()
                    .set_in_path(SharedString::from(
                        path.unwrap_or("".into()).as_os_str().to_str().unwrap_or(""),
                    ));
            }
            in_window.set_button_enabled(
                !in_window.get_in_path().is_empty() && !in_window.get_out_path().is_empty(),
            );
        });
    }
    {
        let weak_window = window.as_weak();
        window.on_button_pressed(move || {
            let in_window = Mutex::new(weak_window.upgrade().unwrap());
            in_window.lock().unwrap().set_interface_enabled(false);
            let in_file = in_window.lock().unwrap().get_in_path().to_string();
            let out_file = in_window.lock().unwrap().get_out_path().to_string();
            let out_file_txt = out_file.clone();
            let separator = in_window
                .lock()
                .unwrap()
                .get_separator()
                .to_string()
                .chars()
                .next()
                .unwrap_or(' ');
            in_window
                .lock()
                .unwrap()
                .set_status_text("Computing VCD statystical analysis...".into());
            thread::spawn(move || {
                perform_analysis(Configuration {
                    in_file,
                    out_file,
                    separator,
                    use_spinner: false,
                });
            });
            in_window.lock().unwrap().set_interface_enabled(true);
            in_window
                .lock()
                .unwrap()
                .set_status_text(format!("VCD analyzed. Result in {}", out_file_txt).into());
        });
    }
    {
        let weak_window = window.as_weak();
        window.on_in_file_edited(move |text| {
            weak_window.upgrade().unwrap().set_button_enabled(
                !text.is_empty() && !weak_window.upgrade().unwrap().get_out_path().is_empty(),
            );
        });
    }
    {
        let weak_window = window.as_weak();
        window.on_out_file_edited(move |text| {
            weak_window.upgrade().unwrap().set_button_enabled(
                !text.is_empty() && !weak_window.upgrade().unwrap().get_in_path().is_empty(),
            );
        });
    }

    window.run().unwrap();
}
