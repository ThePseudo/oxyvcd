use std::{
    collections::HashMap,
    sync::{
        mpsc::{self, Receiver},
        Arc, RwLock,
    },
    thread,
};

use vcd_reader::{Change, LineInfo, SignalValue, VCDFile};

pub struct Configuration {
    pub in_file: String,
    pub separator: char,
}

pub fn index(configuration: Configuration, vcd: Arc<RwLock<VCD>>) -> Result<(), String> {
    let (tx, rx) = mpsc::sync_channel(1000000);
    let vcd1 = vcd.clone();
    let th = thread::spawn(move || translate_infos(rx, vcd1));
    let reader_config = vcd_reader::Configuration {
        in_file: &configuration.in_file,
        separator: configuration.separator,
    };
    VCDFile::new(reader_config).for_each(|info| {
        tx.send(info).unwrap();
    });
    drop(tx);
    th.join().unwrap()
}

#[derive(Debug, Default)]
struct InfoTranslator {
    current_module_index: usize,
}

#[derive(Debug)]
pub enum Node {
    Module(usize),
    Signal(usize),
}

#[derive(Debug, Default)]
pub struct Module {
    pub parent: usize,
    pub children: HashMap<Arc<str>, Node>,
}

#[derive(Debug)]
pub struct Signal {
    pub id: Arc<str>,
    pub sub_id: u16,
    pub name: Arc<str>,
    pub parent_index: usize,
    pub states: Vec<State>,
}

#[derive(Debug, Clone, Copy)]
pub struct State {
    pub value: SignalValue,
    pub time: i64,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug)]
pub struct VCD {
    pub hierarchy: Vec<Module>,
    pub signals: Vec<Signal>,
    pub signals_by_id: HashMap<Arc<str>, usize>,
}

impl Default for State {
    fn default() -> Self {
        State {
            value: Default::default(),
            time: -1,
        }
    }
}

impl Signal {
    fn add_change(&mut self, s: State) {
        self.states.push(s);
    }
}

impl VCD {
    fn push(&mut self, signal: vcd_reader::Signal, translator: &InfoTranslator) {
        for sub_id in 0..signal.num_values {
            let mut name = String::default();
            if signal.num_values > 1 {
                name = String::from(&*name) + &format!("[{}]", sub_id);
            }
            let s = Signal {
                id: String::from(&*signal.id).into(),
                sub_id: sub_id.try_into().unwrap(),
                name: name.into(),
                states: Default::default(),
                parent_index: translator.current_module_index,
            };
            let index = self.signals.len();
            self.signals.push(s);
            let id = self.signals.last().unwrap().id.clone();
            let name = self.signals.last().unwrap().id.clone();
            if sub_id == 0 {
                self.signals_by_id.insert(id.clone(), index);
            }
            self.hierarchy[translator.current_module_index]
                .children
                .insert(name, Node::Signal(index));
        }
    }

    fn add_change(&mut self, change: Change, time: i64) {
        let signal_index = self.signals_by_id.get(&*change.signal_id).unwrap();
        change
            .values
            .into_iter()
            .enumerate()
            .for_each(|(sub_id, state)| {
                self.signals[signal_index + sub_id].add_change(State {
                    value: SignalValue::from(state),
                    time,
                })
            })
    }
}

fn translate_changes(vcd: Arc<RwLock<VCD>>, infos: Receiver<LineInfo>) -> Result<(), String> {
    let mut current_timestamp: i64 = -1;
    for info in infos.into_iter() {
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
            LineInfo::EndInitializations => {}
            LineInfo::Useless => {}
            LineInfo::Dumpports => {}
            LineInfo::ParsingError(s) => {
                return Err(format!("ERROR: found unrecognized symbol: {}", s))
            }

            LineInfo::Timestamp(t) => current_timestamp = t as i64,
            LineInfo::Change(c) => vcd.write().unwrap().add_change(c, current_timestamp),
        }
    }
    Ok(())
}

fn translate_definitions(
    vcd: Arc<RwLock<VCD>>,
    infos: Receiver<LineInfo>,
) -> Result<Receiver<LineInfo>, String> {
    let mut translator = InfoTranslator::default();
    for info in infos.iter() {
        match info {
            LineInfo::Signal(s) => vcd.write().unwrap().push(s, &translator),
            LineInfo::DateInfo(s) => println!("Date: {}", s.trim().replace("$end", "").trim()),
            LineInfo::VersionInfo(s) => {
                println!("Tool: {}", s.trim().replace("$end", "").trim())
            }
            LineInfo::TimeScaleInfo(s) => {
                println!("Time scale: {}", s.trim().replace("$end", "").trim())
            }
            LineInfo::InScope(module) => {
                // Gather last value index
                let mut vcd_lock = vcd.write().unwrap();
                let last_value = vcd_lock.hierarchy.len();
                // Create and push the new module
                let m = Module {
                    parent: translator.current_module_index,
                    ..Module::default()
                };
                vcd_lock.hierarchy.push(m);
                // Update old module children
                vcd_lock.hierarchy[translator.current_module_index]
                    .children
                    .insert(module.into(), Node::Module(last_value));
                // Update current module
                translator.current_module_index = last_value;
            }
            LineInfo::UpScope => {
                translator.current_module_index =
                    vcd.write().unwrap().hierarchy[translator.current_module_index].parent;
            }
            LineInfo::ParsingError(s) => {
                return Err(format!("ERROR: unrecognized symbol {}", s));
            }
            LineInfo::EndDefinitions => break,
            LineInfo::Useless => {}
            LineInfo::Dumpports => panic!("Not expected dumpports here!"),
            LineInfo::Timestamp(t) => panic!("Unexpected timestamp: {:?}", t),
            LineInfo::Change(c) => panic!("Unexpected change: {:?}", c),
            LineInfo::EndInitializations => {
                panic!("End initializations found before the beginning!")
            }
        }
    }
    Ok(infos)
}

fn translate_infos(mut infos: Receiver<LineInfo>, vcd: Arc<RwLock<VCD>>) -> Result<(), String> {
    match translate_definitions(vcd.clone(), infos) {
        Ok(info) => infos = info,
        Err(s) => return Err(s),
    }
    match translate_changes(vcd, infos) {
        Ok(_) => Ok(()),
        Err(s) => Err(s),
    }
}
