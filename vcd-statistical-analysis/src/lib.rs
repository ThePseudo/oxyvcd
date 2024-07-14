use spinners::{Spinner, Spinners};
use std::{
    collections::HashMap,
    rc::Rc,
    sync::mpsc::{self, Receiver},
    thread,
    time::Instant,
};
use vcd_reader::{Change, SignalValue};
use vcd_reader::{LineInfo, VCDFile};

pub fn perform_analysis(file_name: &str) {
    let (tx, rx) = mpsc::sync_channel(1000000);
    let th = thread::spawn(move || {
        translate_infos(rx);
    });
    VCDFile::new(file_name).into_iter().for_each(|info| {
        tx.send(info).unwrap();
    });
    drop(tx);
    th.join().unwrap();
}

struct InfoTranslator {
    modules: Vec<String>,
}

#[derive(Debug)]
struct State {
    value: SignalValue,
    time: i64,
}

impl Default for State {
    fn default() -> Self {
        State {
            value: Default::default(),
            time: -1,
        }
    }
}

#[derive(Debug)]
struct Signal {
    id: Rc<str>,
    sub_id: u16,
    name: Box<str>,
    states: [State; 4],
}

impl Signal {
    fn add_change(&self, state: State) {}
}

#[derive(Default, Debug)]
struct VCD {
    signals: Vec<Signal>,
    signals_by_id: HashMap<Rc<str>, usize>,
}

impl VCD {
    fn push(&mut self, signal: vcd_reader::Signal, translator: &InfoTranslator) {
        let mut modules = translator.modules.join("/");
        modules.push_str(&signal.name);
        for sub_id in 0..signal.num_values {
            let mut name = modules.clone();
            if signal.num_values > 1 {
                name.push_str(&format!("{}", sub_id));
            }
            let s = Signal {
                id: signal.id.clone().into(),
                sub_id: sub_id.try_into().unwrap(),
                name: name.into(),
                states: Default::default(),
            };
            let index = self.signals.len();
            self.signals.push(s);
            let id = self.signals.last().unwrap().id.clone();
            if sub_id == 0 {
                self.signals_by_id.insert(id.clone(), index);
            }
        }
    }

    fn get_signal(&mut self, id: &str, sub_id: usize) -> &mut Signal {
        &mut self.signals[self.signals_by_id.get(id).unwrap() + sub_id]
    }

    fn add_change(&mut self, change: Change, time: i64) {
        change
            .values
            .into_iter()
            .enumerate()
            .for_each(|(sub_id, state)| {
                self.get_signal(&change.signal_id, sub_id)
                    .add_change(State {
                        value: SignalValue::from(state),
                        time,
                    })
            })
    }

    fn translate_changes(&mut self, infos: Receiver<LineInfo>) {
        println!("");
        let mut current_timestamp: i64 = -1;
        for info in infos.into_iter() {
            println!("{:?}", info);
            match info {
                LineInfo::Signal(_) => unreachable!("Error: Signal declaration in initialization"),
                LineInfo::DateInfo(_) => unreachable!("Error: Date info not expected here"),
                LineInfo::VersionInfo(_) => unreachable!("Error: Version info not expected here"),
                LineInfo::TimeScaleInfo(_) => {
                    unreachable!("Error: Time scale info not expected here")
                }
                LineInfo::InScope(_) => unreachable!("Error: Scope definitions not expected here"),
                LineInfo::UpScope => unreachable!("Error: Upscope not expected here"),
                LineInfo::EndDefinitions => {
                    unreachable!("Error: Definitions should have already ended")
                }
                LineInfo::EndInitializations => {
                    unreachable!("Error: Initializations should have already ended")
                }
                LineInfo::Useless => {}
                LineInfo::ParsingError(s) => {
                    println!("{}", s);
                    break;
                }

                LineInfo::Timestamp(t) => current_timestamp = t as i64,
                LineInfo::Change(c) => self.add_change(c, current_timestamp),
            }
        }
    }

    fn translate_initializations(&mut self, infos: Receiver<LineInfo>) -> Receiver<LineInfo> {
        println!("");
        let mut current_timestamp: i64 = 0;
        let start = Instant::now();
        let mut sp = Spinner::new(Spinners::Aesthetic, "Reading signal initializations".into());
        for info in infos.iter() {
            match info {
                LineInfo::Signal(_) => unreachable!("Error: Signal declaration in initialization"),
                LineInfo::DateInfo(_) => unreachable!("Error: Date info not expected here"),
                LineInfo::VersionInfo(_) => unreachable!("Error: Version info not expected here"),
                LineInfo::TimeScaleInfo(_) => {
                    unreachable!("Error: Time scale info not expected here")
                }
                LineInfo::InScope(_) => unreachable!("Error: Scope definitions not expected here"),
                LineInfo::UpScope => unreachable!("Error: Upscope not expected here"),
                LineInfo::EndDefinitions => {
                    unreachable!("Error: Definitions should have already ended")
                }
                LineInfo::Useless => {}
                LineInfo::ParsingError(s) => {
                    println!("{}", s);
                    break;
                }
                LineInfo::EndInitializations => {
                    sp.stop_with_message(String::from("Signals initialized correctly"));
                    break;
                }
                LineInfo::Timestamp(t) => current_timestamp = t as i64,
                LineInfo::Change(c) => {
                    c.values.into_iter().enumerate().for_each(|(index, value)| {
                        let signal = self.get_signal(&c.signal_id, index);
                        signal.states[0] = State {
                            value: SignalValue::from(value),
                            time: current_timestamp,
                        }
                    })
                }
            }
        }
        let end = Instant::now();
        println!("Duration: {} s", (end - start).as_millis() as f64 / 1000.0);
        infos
    }

    fn translate_definitions(&mut self, infos: Receiver<LineInfo>) -> Receiver<LineInfo> {
        println!("");
        let mut sp = Spinner::new(Spinners::Aesthetic, "Reading signal declarations".into());
        let start = Instant::now();
        let mut translator = InfoTranslator { modules: vec![] };
        for info in infos.iter() {
            match info {
                LineInfo::Signal(s) => self.push(s, &translator),
                LineInfo::DateInfo(s) => println!("Date: {}", s.trim().replace("$end", "").trim()),
                LineInfo::VersionInfo(s) => {
                    println!("Tool: {}", s.trim().replace("$end", "").trim())
                }
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
                        self.signals.len()
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
        infos
    }
}

fn translate_infos(mut infos: Receiver<LineInfo>) {
    let mut vcd = VCD::default();
    infos = vcd.translate_definitions(infos);
    infos = vcd.translate_initializations(infos);
    vcd.translate_changes(infos);
}
