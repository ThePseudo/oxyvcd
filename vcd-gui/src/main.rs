use vcd_statistical_analysis::{perform_analysis, Configuration};

slint::slint! {
    import { Button, GroupBox, LineEdit } from "std-widgets.slint";

    export component MainWindow inherits Window {
        out property<string> in_path <=> inpath.text;
        out property<string> out_path <=> outpath.text;
        out property<string> separator <=> sep.text;

        callback button-pressed <=> evaluate_button.clicked;
        preferred-width: 640px;
        preferred-height: 480px;
        min-width: 420px;
        min-height: 240px;
        title: "vcd";
        VerticalLayout {
            padding: 8px;
            alignment: center;
                GroupBox {
                    Text {
                        vertical-alignment: center;
                        text: "Input VCD file path  ";
                    }
                    inpath := LineEdit {
                        placeholder-text: "Path";
                    }
                }
                GroupBox {
                    Text {
                        vertical-alignment: center;
                        text: "Output file path  ";
                    }
                    outpath := LineEdit {
                        placeholder-text: "Path";
                    }
                }
                GroupBox {
                    Text {
                        vertical-alignment: center;
                        text: "Separator character  ";
                    }
                    sep := LineEdit {
                        placeholder-text: "Separator";
                        text: "<";
                    }
                }
                evaluate_button := Button {
                    text: "Perform analysis";
                }
        }
    }
}
fn main() {
    let window = MainWindow::new().unwrap();
    let weak_window = window.as_weak();
    window.on_button_pressed(move || {
        let in_window = weak_window.upgrade().unwrap();
        let in_file = in_window.get_in_path().to_string();
        let out_file = in_window.get_out_path().to_string();
        let separator = in_window
            .get_separator()
            .to_string()
            .chars()
            .next()
            .unwrap();

        perform_analysis(Configuration {
            in_file,
            out_file,
            separator,
            use_spinner: false,
        });
    });
    window.run().unwrap();
}
