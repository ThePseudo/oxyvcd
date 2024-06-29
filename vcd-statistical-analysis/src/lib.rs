use std::{
    sync::mpsc::{self, Receiver},
    thread,
};

use vcd_reader::vcdfile::{LineInfo, VCDFile};

pub fn perform_analysis(file_name: &str) {
    let (tx, rx) = mpsc::sync_channel(1000000);
    thread::spawn(move || {
        translate_infos(rx);
    });
    VCDFile::new(file_name).into_iter().for_each(|info| {
        tx.send(info).unwrap();
    });
}

struct InfoTranslator {
    modules: Vec<String>,
}

struct Signal {
    id: String,
    sub_id: u16,
    name: String,
}

#[derive(Default)]
struct VCD {
    signals: Vec<Signal>,
}

impl VCD {
    fn push(&mut self, signal: vcd_reader::vcdfile::Signal, translator: &InfoTranslator) {
        let mut modules = translator.modules.join("/");
        modules.push_str(&signal.name);
        for sub_id in 0..signal.num_values {
            let mut s = Signal {
                id: signal.id.clone(),
                sub_id: sub_id.try_into().unwrap(),
                name: modules.clone(),
            };
            if signal.num_values > 1 {
                s.name.push_str(&format!("{}", sub_id));
            }
            self.signals.push(s);
        }
    }
}

fn translate_infos(infos: Receiver<LineInfo>) {
    let mut vcd = VCD::default();
    translate_definitions(&mut vcd, &infos);
    for info in infos {}
}

fn translate_definitions(vcd: &mut VCD, infos: &Receiver<LineInfo>) {
    let mut translator = InfoTranslator { modules: vec![] };
    for info in infos {
        match info {
            LineInfo::Signal(s) => vcd.push(s, &translator),
            LineInfo::DateInfo(s) => println!("Date: {}", s.trim().replace("$end", "").trim()),
            LineInfo::VersionInfo(s) => println!("Tool: {}", s.trim().replace("$end", "").trim()),
            LineInfo::TimeScaleInfo(s) => {
                println!("Time scale: {}", s.trim().replace("$end", "").trim())
            }
            LineInfo::InScope(module) => translator.modules.push(module),
            LineInfo::UpScope => {
                translator.modules.pop().unwrap();
            }
            LineInfo::ParsingError(s) => {
                println!("{}", s);
                break;
            }
            LineInfo::EndDefinitions => {
                println!(
                    "Signals read correctly. Number of signals: {}",
                    vcd.signals.len()
                );
                break;
            }
        }
    }
}
