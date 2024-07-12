use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use spinners::{Spinner, Spinners};
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
        match info {
            LineInfo::Signal(_) => unreachable!("Error: Signal declaration in initialization"),
            LineInfo::DateInfo(_) => unreachable!("Error: Date info not expected here"),
            LineInfo::VersionInfo(_) => unreachable!("Error: Version info not expected here"),
            LineInfo::TimeScaleInfo(_) => unreachable!("Error: Time scale info not expected here"),
            LineInfo::InScope(_) => unreachable!("Error: Scope definitions not expected here"),
            LineInfo::UpScope => unreachable!("Error: Upscope not expected here"),
            LineInfo::ParsingError(s) => {
                println!("{}", s);
                break;
            }
            LineInfo::EndDefinitions => {
                unreachable!("Error: Definitions should have already ended")
            }
            LineInfo::EndInitializations => {
                unreachable!("Error: Initializations should have already ended")
            }
            LineInfo::Timestamp(_) => todo!(),
            LineInfo::Change(_) => todo!(),
            LineInfo::Useless => todo!(),
        }
    }
}

fn translate_initializations(vcd: &mut VCD, infos: &Receiver<LineInfo>) {
    for info in infos {
        match info {
            LineInfo::Signal(_) => unreachable!("Error: Signal declaration in initialization"),
            LineInfo::DateInfo(_) => unreachable!("Error: Date info not expected here"),
            LineInfo::VersionInfo(_) => unreachable!("Error: Version info not expected here"),
            LineInfo::TimeScaleInfo(_) => unreachable!("Error: Time scale info not expected here"),
            LineInfo::InScope(_) => unreachable!("Error: Scope definitions not expected here"),
            LineInfo::UpScope => unreachable!("Error: Upscope not expected here"),
            LineInfo::ParsingError(s) => {
                println!("{}", s);
                break;
            }
            LineInfo::EndDefinitions => {
                unreachable!("Error: Definitions should have already ended")
            }
            LineInfo::Timestamp(_) => todo!(),
            LineInfo::Change(_) => todo!(),
            LineInfo::EndInitializations => {
                break;
            }
            LineInfo::Useless => todo!(),
        }
    }
}

fn translate_definitions(vcd: &mut VCD, infos: &Receiver<LineInfo>) {
    println!("");
    let mut sp = Spinner::new(Spinners::Aesthetic, "Reading signal declarations".into());
    let start = Instant::now();
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
                sp.stop_with_message(format!(
                    "Signals read correctly. Number of signals: {}",
                    vcd.signals.len()
                ));
                break;
            }
            LineInfo::Useless => {}
            LineInfo::Timestamp(t) => panic!("Unexpected timestamp: {:?}", t),
            LineInfo::Change(c) => panic!("Unexpected change: {:?}", c),
            LineInfo::EndInitializations => {
                panic!("End initializations found before the beginning!")
            }
        }
    }
    let end = Instant::now();
    println!("Duration: {} s", (end - start).as_millis() as f64 / 1000.0);
}
