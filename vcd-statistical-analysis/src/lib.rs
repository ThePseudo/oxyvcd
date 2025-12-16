use logger::{Log, Priority};
use spinners::{Spinner, Spinners};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
    rc::Rc,
    sync::mpsc::{self, Receiver},
    thread,
    time::Instant,
};
use vcd_reader::{Change, SignalValue};
use vcd_reader::{LineInfo, VCDFile};

pub struct Configuration {
    pub in_file: String,
    pub out_file: String,
    pub separator: char,
    pub use_spinner: bool,
}

pub fn perform_analysis(c: Configuration) {
    let (tx, rx) = mpsc::sync_channel(1000000);
    let th = thread::spawn(move || translate_infos(rx, c.use_spinner));
    let reader_config = vcd_reader::Configuration {
        in_file: &c.in_file,
        separator: c.separator,
    };
    VCDFile::new(reader_config).for_each(|info| {
        tx.send(info).unwrap();
    });
    drop(tx);
    let vcd = th.join().unwrap();

    let mut writer = BufWriter::new(File::create(c.out_file).unwrap());
    writer
        .write_fmt(format_args!("{}", vcd.to_result_string()))
        .unwrap();
}

struct InfoTranslator {
    modules: Vec<Rc<str>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
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
    name: Vec<Rc<str>>,
    states: [State; 3], // Initial state, opposite state, back to initial state
    initial_state: State,
}

impl Signal {
    fn add_change(&mut self, state: State) {
        if state.value != SignalValue::X {
            match self.states[0].value {
                SignalValue::UP => {
                    if state.value == SignalValue::DOWN && self.states[1].value == SignalValue::X
                    // update once
                    {
                        self.states[1] = state;
                    } else if state.value == SignalValue::UP
                        && self.states[1].value != SignalValue::X // update only when states[2] updated
                        && self.states[2].value == SignalValue::X
                    // update once
                    {
                        self.states[2] = state;
                    }
                }
                SignalValue::DOWN => {
                    if state.value == SignalValue::UP && self.states[1].value == SignalValue::X
                    // update once
                    {
                        self.states[1] = state;
                    } else if state.value == SignalValue::DOWN
                        && self.states[1].value != SignalValue::X // update only when states[2] updated
                        && self.states[2].value == SignalValue::X
                    // update once
                    {
                        self.states[2] = state;
                    }
                }
                _ => {
                    // starts with X or Z
                    self.states[0] = state;
                }
            }
        }
    }

    fn calculate_coverage(&self) -> f32 {
        let up_transition = 0.5 * (self.has_transitioned_up() as u32 as f32);
        let down_transition = 0.5 * (self.has_transitioned_down() as u32 as f32);
        up_transition + down_transition
    }

    fn has_transitioned_up(&self) -> bool {
        match self.states[0].value {
            SignalValue::UP => self.states[2].value != SignalValue::X,
            SignalValue::DOWN => self.states[1].value != SignalValue::X,
            _ => false,
        }
    }

    fn has_transitioned_down(&self) -> bool {
        match self.states[0].value {
            SignalValue::UP => self.states[1].value != SignalValue::X,
            SignalValue::DOWN => self.states[2].value != SignalValue::X,
            _ => false,
        }
    }

    fn to_result_string(&self) -> String {
        let initial_value: char = self.initial_state.value.into();
        format!(
            "{} {}-{} {:.1} {} {} {}",
            self.name.join("/"),
            self.id,
            self.sub_id,
            self.calculate_coverage(),
            self.has_transitioned_up() as u8,
            self.has_transitioned_down() as u8,
            initial_value
        )
    }

    fn result_explanation() -> &'static str {
        "# Signal name, id-sub_id, coverage [%], has transitioned up, has transitioned down, initial value"
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, Debug)]
struct VCD {
    signals: Vec<Signal>,
    signals_by_id: HashMap<Rc<str>, usize>,
}

impl VCD {
    fn push(&mut self, signal: vcd_reader::Signal, translator: &InfoTranslator) {
        let mut modules = translator.modules.clone();
        modules.push(signal.name);
        for sub_id in 0..signal.num_values {
            let mut name = modules.clone();
            if signal.num_values > 1 {
                name.push(format!("[{}]", sub_id).into_boxed_str().into());
            }
            let s = Signal {
                id: signal.id.clone(),
                sub_id: sub_id.try_into().unwrap(),
                name,
                states: Default::default(),
                initial_state: Default::default(),
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

    fn translate_changes(&mut self, infos: Receiver<LineInfo>, use_spinner: bool) {
        let mut current_timestamp: i64 = -1;
        let start = Instant::now();
        Log::write(Priority::Info, "Reading signal changes");
        let sp = match use_spinner {
            true => Some(Spinner::new(Spinners::Aesthetic, "".into())),
            false => None,
        };
        for info in infos.into_iter() {
            match info {
                LineInfo::Signal(_) => unreachable!("Error: Signal declaration in initialization"),
                LineInfo::DateInfo(_) => unreachable!("Error: Date info not expected here"),
                LineInfo::VersionInfo(_) => unreachable!("Error: Version info not expected here"),
                LineInfo::Dumpports => unreachable!("Error: Dumpports not expected here"),
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
                    Log::write(Priority::Error, &s);
                    break;
                }

                LineInfo::Timestamp(t) => current_timestamp = t as i64,
                LineInfo::Change(c) => self.add_change(c, current_timestamp),
            }
        }
        if let Some(mut s) = sp {
            s.stop();
        }
        Log::write(Priority::Info, "Changes read correctly");
        let end = Instant::now();
        Log::write(
            Priority::Info,
            &format!("Duration: {} s", (end - start).as_millis() as f64 / 1000.0),
        );
    }

    fn translate_initializations(
        &mut self,
        infos: Receiver<LineInfo>,
        use_spinner: bool,
    ) -> Receiver<LineInfo> {
        let mut current_timestamp: i64 = 0;
        let start = Instant::now();
        Log::write(Priority::Info, "Reading signal initializations");
        let sp = match use_spinner {
            true => Some(Spinner::new(Spinners::Aesthetic, "".into())),
            false => None,
        };
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
                    Log::write(Priority::Error, &s);
                    break;
                }
                LineInfo::EndInitializations => {
                    if let Some(mut s) = sp {
                        s.stop();
                    }
                    Log::write(Priority::Info, "Signals initialized correctly");
                    break;
                }
                LineInfo::Dumpports => Log::write(Priority::Info, "Dumpports found: VCD ok!"),
                LineInfo::Timestamp(t) => current_timestamp = t as i64,
                LineInfo::Change(c) => {
                    c.values.into_iter().enumerate().for_each(|(index, value)| {
                        let signal = self.get_signal(&c.signal_id, index);
                        signal.states[0] = State {
                            value: SignalValue::from(value),
                            time: current_timestamp,
                        };
                        signal.states[1] = State {
                            value: SignalValue::from(value),
                            time: current_timestamp,
                        };
                        signal.initial_state = State {
                            value: SignalValue::from(value),
                            time: current_timestamp,
                        };
                    })
                }
            }
        }
        let end = Instant::now();
        Log::write(
            Priority::Info,
            &format!("Duration: {} s", (end - start).as_millis() as f64 / 1000.0),
        );
        infos
    }

    fn translate_definitions(
        &mut self,
        infos: Receiver<LineInfo>,
        use_spinner: bool,
    ) -> Receiver<LineInfo> {
        Log::write(Priority::Info, "Reading signal declarations");
        let sp = match use_spinner {
            true => Some(Spinner::new(Spinners::Aesthetic, "".into())),
            false => None,
        };
        let start = Instant::now();
        let mut translator = InfoTranslator { modules: vec![] };
        for info in infos.iter() {
            match info {
                LineInfo::Signal(s) => self.push(s, &translator),
                LineInfo::DateInfo(s) => Log::write(
                    Priority::Info,
                    &format!("Date: {}", s.trim().replace("$end", "").trim()),
                ),
                LineInfo::VersionInfo(s) => Log::write(
                    Priority::Info,
                    &format!("Tool: {}", s.trim().replace("$end", "").trim()),
                ),
                LineInfo::TimeScaleInfo(s) => Log::write(
                    Priority::Info,
                    &format!("Time scale: {}", s.trim().replace("$end", "").trim()),
                ),
                LineInfo::InScope(module) => {
                    translator.modules.push(module.into_boxed_str().into())
                }
                LineInfo::UpScope => {
                    translator.modules.pop().unwrap();
                }
                LineInfo::ParsingError(s) => {
                    Log::write(Priority::Error, &s);
                    break;
                }
                LineInfo::EndDefinitions => {
                    if let Some(mut s) = sp {
                        s.stop();
                    }
                    Log::write(
                        Priority::Info,
                        &format!(
                            "Signals read correctly. Number of signals: {}",
                            self.signals.len()
                        ),
                    );
                    break;
                }
                LineInfo::Useless => {}
                LineInfo::Dumpports => unreachable!("Error: Dumpports not expected here"),
                LineInfo::Timestamp(t) => panic!("Unexpected timestamp: {:?}", t),
                LineInfo::Change(c) => panic!("Unexpected change: {:?}", c),
                LineInfo::EndInitializations => {
                    panic!("End initializations found before the beginning!")
                }
            }
        }
        let end = Instant::now();
        Log::write(
            Priority::Info,
            &format!("Duration: {} s", (end - start).as_millis() as f64 / 1000.0),
        );
        infos
    }

    pub fn to_result_string(&self) -> String {
        let mut total_coverage: f64 = self
            .signals
            .iter()
            .map(|signal| signal.calculate_coverage() as f64)
            .sum();
        total_coverage /= self.signals.len() as f64;
        let explanation = format!(
            "# VCD Statistical analysis. Total coverage: {:.2} % over {} signals\n{}\n",
            total_coverage * 100.0,
            self.signals.len(),
            Signal::result_explanation()
        );
        let result_values: Vec<String> = self
            .signals
            .iter()
            .map(|signal| signal.to_result_string())
            .collect();
        format!("{}{}", explanation, result_values.join("\n"))
    }
}

fn translate_infos(mut infos: Receiver<LineInfo>, use_spinner: bool) -> VCD {
    let mut vcd = VCD::default();
    infos = vcd.translate_definitions(infos, use_spinner);
    infos = vcd.translate_initializations(infos, use_spinner);
    vcd.translate_changes(infos, use_spinner);
    vcd
}

unsafe impl Send for Signal {}
unsafe impl Send for VCD {}
