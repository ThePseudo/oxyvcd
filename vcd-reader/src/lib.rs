use futures::executor::block_on;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    rc::Rc,
    str::SplitAsciiWhitespace,
};
pub struct Configuration<'vcd> {
    pub in_file: &'vcd str,
    pub separator: char,
}

#[derive(Debug, Clone, Copy)]
pub enum SignalType {
    Bus,
    Gate,
}

pub struct VCDFile {
    reader: BufReader<File>,
    line: String,
    lineno: usize,
    part: Part,
    separator: char,
    signals: HashMap<Rc<str>, Signal>,
}

enum Part {
    Declarations,
    Initializations,
    Changes,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub id: Rc<str>,
    pub name: Rc<str>,
    pub num_values: usize,
    pub signal_type: SignalType,
}

#[derive(Debug)]
pub struct Change {
    pub signal_id: String,
    pub values: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum SignalValue {
    UP,
    DOWN,
    X,
    Z,
}

impl Default for SignalValue {
    fn default() -> Self {
        Self::X
    }
}

#[derive(Debug)]
pub enum LineInfo {
    Signal(Signal),
    Timestamp(usize),
    Change(Change),
    DateInfo(String),
    VersionInfo(String),
    TimeScaleInfo(String),
    InScope(String),
    UpScope,
    ParsingError(String),
    EndDefinitions,
    EndInitializations,
    Useless,
}

impl VCDFile {
    pub fn new(configuration: Configuration) -> Self {
        VCDFile {
            reader: BufReader::new(File::open(configuration.in_file).unwrap_or_else(|err| {
                panic!(
                    "FATAL ERROR: File {} could not be opened. Reason: {}",
                    configuration.in_file, err
                )
            })),
            line: Default::default(),
            lineno: 0,
            part: Part::Declarations,
            separator: configuration.separator,
            signals: HashMap::new(),
        }
    }

    fn read_line(&mut self) -> usize {
        self.line.clear();
        self.lineno += 1;
        self.reader.read_line(&mut self.line).unwrap_or_else(|err| {
            panic!(
                "FATAL ERROR: an error occurred during the file reading: {}",
                err
            )
        })
    }

    fn read_line_noclear(&mut self) -> usize {
        self.lineno += 1;
        self.reader.read_line(&mut self.line).unwrap_or_else(|err| {
            panic!(
                "FATAL ERROR: an error occurred during the file reading: {}",
                err
            )
        })
    }

    fn unrecognized_symbol(symbol: &str, lineno: usize) -> Option<LineInfo> {
        Some(LineInfo::ParsingError(format!(
            "FATAL ERROR: unrecoginzed symbol {} at line {}",
            symbol, lineno
        )))
    }

    fn unexpected_eof(lineno: usize) -> Option<LineInfo> {
        Some(LineInfo::ParsingError(format!(
            "FATAL ERROR: unexpected end of file at line {}",
            lineno
        )))
    }

    fn manage_in_scope(
        &self,
        scope_type: &str,
        mut words: SplitAsciiWhitespace,
    ) -> Option<LineInfo> {
        match scope_type {
            "module" | "task" => match words.next() {
                Some(scope_name) => Some(LineInfo::InScope(String::from(scope_name))),
                None => Self::unexpected_eof(self.lineno),
            },
            _ => Self::unrecognized_symbol(scope_type, self.lineno),
        }
    }

    fn manage_var_type(
        &mut self,
        var_type: &str,
        split_line: SplitAsciiWhitespace,
    ) -> Option<LineInfo> {
        match var_type {
            "port" => self.manage_var_port(split_line),
            "wire" => self.manage_var_wire(split_line),
            _ => Self::unrecognized_symbol(var_type, self.lineno),
        }
    }

    fn manage_var_wire(&mut self, mut split_line: SplitAsciiWhitespace) -> Option<LineInfo> {
        let mut s = Signal {
            num_values: 1,
            name: String::default().into(),
            id: String::default().into(),
            signal_type: SignalType::Gate,
        };
        match split_line.next() {
            Some(quantity_str) => {
                if quantity_str != "1" {
                    s.signal_type = SignalType::Bus;
                    s.num_values = quantity_str.parse().unwrap(); // Inside here should save parsing operations
                }
            }
            None => return Self::unexpected_eof(self.lineno),
        }
        match split_line.next() {
            Some(mut id) => {
                s.id = {
                    if id.starts_with(self.separator) {
                        id = &id[1..];
                    }
                    String::from(id).into()
                }
            }
            None => return Self::unexpected_eof(self.lineno),
        }
        match split_line.next() {
            Some(name) => s.name = String::from(name).into(),
            None => return Self::unexpected_eof(self.lineno),
        }
        self.signals.insert(s.id.clone(), s.clone());
        Some(LineInfo::Signal(s))
    }

    fn manage_var_port(&mut self, mut split_line: SplitAsciiWhitespace) -> Option<LineInfo> {
        let mut s = Signal {
            num_values: 1,
            name: String::default().into(),
            id: String::default().into(),
            signal_type: SignalType::Gate,
        };
        match split_line.next() {
            Some(quantity_str) => {
                if quantity_str != "1" {
                    s.signal_type = SignalType::Bus;
                    let size_str = &quantity_str[1..quantity_str.len() - 1];
                    let mut start_end_split = size_str.split(':');
                    let start: i32 = match start_end_split.next() {
                        Some(value) => value.parse().unwrap(),
                        None => return Self::unexpected_eof(self.lineno),
                    };
                    let end: i32 = match start_end_split.next() {
                        Some(value) => value.parse().unwrap(),
                        None => return Self::unexpected_eof(self.lineno),
                    };
                    s.num_values = (i32::abs(end - start) + 1) as usize;
                }
            }
            None => return Self::unexpected_eof(self.lineno),
        }
        match split_line.next() {
            Some(mut id) => {
                s.id = {
                    if id.starts_with(self.separator) {
                        id = &id[1..];
                    }
                    String::from(id).into()
                }
            }
            None => return Self::unexpected_eof(self.lineno),
        }
        match split_line.next() {
            Some(name) => s.name = String::from(name).into(),
            None => return Self::unexpected_eof(self.lineno),
        }
        self.signals.insert(s.id.clone(), s.clone());
        Some(LineInfo::Signal(s))
    }

    fn next_declarations(&mut self, line_slice: String) -> Option<LineInfo> {
        let mut split_line = line_slice.split_ascii_whitespace();
        match split_line.next() {
            Some(string) => match string {
                "$date" => {
                    self.line = self.line.replace("$date", "");
                    while !self.line.contains("$end") {
                        self.read_line_noclear();
                    }
                    Some(LineInfo::DateInfo(self.line.clone()))
                }
                "$version" => {
                    self.line = self.line.replace("$version", "");
                    while !self.line.contains("$end") {
                        self.read_line_noclear();
                    }
                    Some(LineInfo::VersionInfo(self.line.clone()))
                }
                "$timescale" => {
                    self.line = self.line.replace("$timescale", "");
                    while !self.line.contains("$end") {
                        self.read_line_noclear();
                    }
                    Some(LineInfo::TimeScaleInfo(self.line.clone()))
                }
                "$scope" => match split_line.next() {
                    Some(scope_type) => self.manage_in_scope(scope_type, split_line),
                    None => Self::unexpected_eof(self.lineno),
                },
                "$upscope" => Some(LineInfo::UpScope),
                "$var" => match split_line.next() {
                    Some(var_type) => self.manage_var_type(var_type, split_line),
                    None => Self::unexpected_eof(self.lineno),
                },
                "$enddefinitions" => {
                    self.part = Part::Initializations;
                    Some(LineInfo::EndDefinitions)
                }
                "$end" => Some(LineInfo::Useless),
                _ => Self::unrecognized_symbol(string, self.lineno),
            },
            None => {
                unreachable!("WARNING: Empty line passed filtering! This should not have happened!")
            }
        }
    }

    fn next_initializations(&mut self, line_slice: String) -> Option<LineInfo> {
        match line_slice.as_ref() {
            "$dumpports" => Some(LineInfo::Useless),
            "$end" => {
                self.part = Part::Changes;
                Some(LineInfo::EndInitializations)
            }
            _ => {
                if let Some(time_str) = line_slice.strip_prefix('#') {
                    Some(LineInfo::Timestamp(time_str.parse().unwrap()))
                } else {
                    let mut starts_p = false;
                    let mut values = &line_slice[0..];
                    if line_slice.starts_with('b') {
                        values = &line_slice[1..];
                    } else if line_slice.starts_with('p') {
                        starts_p = true;
                        values = &line_slice[1..];
                    }
                    let mut line_parts = values.split(self.separator);
                    let values = line_parts.next().unwrap();
                    if starts_p && self.separator == ' ' {
                        line_parts.next().unwrap();
                        line_parts.next().unwrap();
                    }
                    let signal_id = line_parts.next().unwrap();
                    Some(LineInfo::Change(Change {
                        signal_id: String::from(signal_id),
                        values: values.into(),
                    }))
                }
            }
        }
    }

    fn next_changes(&mut self, line_slice: String) -> Option<LineInfo> {
        if let Some(time_str) = line_slice.strip_prefix('#') {
            Some(LineInfo::Timestamp(time_str.parse().unwrap()))
        } else {
            let mut starts_p = false;
            let mut values = &line_slice[0..];
            if line_slice.starts_with('b') {
                values = &line_slice[1..];
            } else if line_slice.starts_with('p') {
                starts_p = true;
                values = &line_slice[1..];
            }
            let mut line_parts = values.split(self.separator);
            let values = line_parts.next().unwrap();
            if starts_p && self.separator == ' ' {
                line_parts.next().unwrap();
                line_parts.next().unwrap();
            }
            let signal_id = line_parts.next().unwrap();
            Some(LineInfo::Change(Change {
                signal_id: String::from(signal_id),
                values: values.into(),
            }))
        }
    }

    async fn next_line(&mut self) -> Option<String> {
        let mut line_slice = String::from("");
        // Skip empty lines
        while line_slice.is_empty() {
            if self.read_line() == 0 {
                return None;
            }
            line_slice = self.line.trim().to_owned();
        }
        Some(line_slice)
    }
}

impl Iterator for VCDFile {
    type Item = LineInfo;

    fn next(&mut self) -> Option<Self::Item> {
        match block_on(self.next_line()) {
            Some(line_slice) => match self.part {
                Part::Declarations => self.next_declarations(line_slice),
                Part::Initializations => self.next_initializations(line_slice),
                Part::Changes => self.next_changes(line_slice),
            },
            None => None,
        }
    }
}

impl From<u8> for SignalValue {
    fn from(val: u8) -> Self {
        match val {
            b'D' | b'd' | b'L' | b'l' | b'0' => SignalValue::DOWN,
            b'U' | b'u' | b'H' | b'h' | b'1' => SignalValue::UP,
            b'F' | b'Z' | b'T' | b'z' => SignalValue::Z,
            _ => SignalValue::X,
        }
    }
}

impl From<SignalValue> for char {
    fn from(val: SignalValue) -> Self {
        match val {
            SignalValue::UP => '1',
            SignalValue::DOWN => '0',
            SignalValue::X => 'x',
            SignalValue::Z => 'z',
        }
    }
}

unsafe impl Send for Signal {}
