use vcd_statistical_analysis::{perform_analysis, Configuration};

slint::slint! {
    import { Button, GroupBox, LineEdit } from "std-widgets.slint";

    export component MainWindow inherits Window {
        out property<string> in_path <=> inpath.text;
        out property<string> out_path <=> outpath.text;
        out property<string> separator <=> sep.text;
        in property<bool> interface_enabled;

        callback button-pressed <=> evaluate_button.clicked;
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
                        text: "Separator character (default: '<')";
                    }
                    sep := LineEdit {
                        placeholder-text: "Separator";
                        text: "<";
                        read-only: !interface_enabled;
                    }
                }
            }
            evaluate_button := Button {
                text: "Perform analysis";
                enabled: interface_enabled;
                primary: true;
            }
        }
    }
}

fn main() {
    let window = MainWindow::new().unwrap();
    let weak_window = window.as_weak();
    window.set_interface_enabled(true);
    window.on_button_pressed(move || {
        let in_window = weak_window.upgrade().unwrap();
        in_window.set_interface_enabled(false);
        let in_file = in_window.get_in_path().to_string();
        let out_file = in_window.get_out_path().to_string();
        let separator = in_window
            .get_separator()
            .to_string()
            .chars()
            .next()
            .unwrap_or('<');
        perform_analysis(Configuration {
            in_file,
            out_file,
            separator,
            use_spinner: false,
        });
        in_window.set_interface_enabled(true);
    });
    window.run().unwrap();
}
