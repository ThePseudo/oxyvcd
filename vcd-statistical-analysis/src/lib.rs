use std::{
    sync::mpsc::{self, Receiver},
    thread,
};

use vcd_reader::{LineInfo, VCDFile};

pub fn perform_analysis(file_name: &str) {
    let (tx, rx) = mpsc::sync_channel(1000000);
    let th = thread::spawn(move || {
        translate_infos(rx);
    });
    VCDFile::new(file_name).into_iter().for_each(|info| {
        tx.send(info).unwrap();
    });
    th.join().unwrap();
}

struct InfoTranslator {
    modules: Vec<String>,
}

enum SignalValue {
    UP,
    DOWN,
    X,
    Z,
}

struct Signal {
    id: String,
    sub_id: u16,
    name: String,
    initial_value: SignalValue,
}

#[derive(Default)]
struct VCD {
    signals: Vec<Signal>,
}

impl VCD {
    fn push(&mut self, signal: vcd_reader::Signal, translator: &InfoTranslator) {
        let mut modules = translator.modules.join("/");
        modules.push_str(&signal.name);
        for sub_id in 0..signal.num_values {
            let mut s = Signal {
                id: signal.id.clone(),
                sub_id: sub_id.try_into().unwrap(),
                name: modules.clone(),
                initial_value: SignalValue::X,
            };
            if signal.num_values > 1 {
                s.name.push_str(&format!("{}", s.sub_id));
            }
            self.signals.push(s);
        }
    }
}

fn translate_infos(infos: Receiver<LineInfo>) {
    let mut vcd = VCD::default();
    translate_definitions(&mut vcd, &infos);
    translate_initializations(&mut vcd, &infos);
    translate_changes(&mut vcd, &infos);
}

fn translate_changes(vcd: &mut VCD, infos: &Receiver<LineInfo>) {
    for info in infos {
        //match info {
        //    LineInfo::Signal(_) => todo!(),
        //    LineInfo::DateInfo(_) => todo!(),
        //    LineInfo::VersionInfo(_) => todo!(),
        //    LineInfo::TimeScaleInfo(_) => todo!(),
        //    LineInfo::InScope(_) => todo!(),
        //    LineInfo::UpScope => todo!(),
        //    LineInfo::ParsingError(_) => todo!(),
        //    LineInfo::EndDefinitions => todo!(),
        //}
    }
}

fn translate_initializations(vcd: &mut VCD, infos: &Receiver<LineInfo>) {
    for info in infos {
        //match info {
        //    LineInfo::Signal(_) => todo!(),
        //    LineInfo::DateInfo(_) => todo!(),
        //    LineInfo::VersionInfo(_) => todo!(),
        //    LineInfo::TimeScaleInfo(_) => todo!(),
        //    LineInfo::InScope(_) => todo!(),
        //    LineInfo::UpScope => todo!(),
        //    LineInfo::ParsingError(_) => todo!(),
        //    LineInfo::EndDefinitions => todo!(),
        //}
    }
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
            LineInfo::Useless => {}
        }
    }
}
