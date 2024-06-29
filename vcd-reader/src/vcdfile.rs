use std::{
    fs::File,
    io::{BufRead, BufReader},
    str::SplitAsciiWhitespace,
};

pub struct VCDFile {
    reader: BufReader<File>,
    line: String,
    lineno: usize,
}

#[derive(Debug)]
pub struct Signal {
    pub id: String,
    pub name: String,
    pub num_values: usize,
}

#[derive(Debug)]
pub enum LineInfo {
    Signal(Signal),
    DateInfo(String),
    VersionInfo(String),
    TimeScaleInfo(String),
    InScope(String),
    UpScope,
    ParsingError(String),
    EndDefinitions,
}

impl VCDFile {
    pub fn new(file_name: &str) -> Self {
        VCDFile {
            reader: BufReader::new(File::open(file_name).unwrap_or_else(|err| {
                panic!(
                    "FATAL ERROR: File {} could not be opened. Reason: {}",
                    file_name, err
                )
            })),
            line: Default::default(),
            lineno: 0,
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
        &self,
        var_type: &str,
        split_line: SplitAsciiWhitespace,
    ) -> Option<LineInfo> {
        match var_type {
            "port" => self.manage_var_port(split_line),
            "wire" => todo!(),
            _ => Self::unrecognized_symbol(var_type, self.lineno),
        }
    }

    fn manage_var_port(&self, mut split_line: SplitAsciiWhitespace) -> Option<LineInfo> {
        let mut s = Signal {
            num_values: 1,
            name: String::default(),
            id: String::default(),
        };
        match split_line.next() {
            Some(quantity_str) => {
                if quantity_str != "1" {
                    let size_str = &quantity_str[1..quantity_str.len() - 1];
                    let mut start = 0;
                    let mut end = 0;
                    let mut start_end_split = size_str.split(":");
                    match start_end_split.next() {
                        Some(value) => start = value.parse().unwrap(),
                        None => return Self::unexpected_eof(self.lineno),
                    };
                    match start_end_split.next() {
                        Some(value) => end = value.parse().unwrap(),
                        None => return Self::unexpected_eof(self.lineno),
                    };
                    s.num_values = i32::abs(end - start) as usize;
                }
            }
            None => return Self::unexpected_eof(self.lineno),
        }
        match split_line.next() {
            Some(id) => s.id = String::from(id),
            None => return Self::unexpected_eof(self.lineno),
        }
        match split_line.next() {
            Some(name) => s.name = String::from(name),
            None => return Self::unexpected_eof(self.lineno),
        }
        Some(LineInfo::Signal(s))
    }
}

impl Iterator for VCDFile {
    type Item = LineInfo;

    fn next(&mut self) -> Option<Self::Item> {
        // Skip empty lines
        let mut line_slice = "";
        while line_slice.len() == 0 {
            if self.read_line() == 0 {
                return None;
            }
            line_slice = self.line.trim();
        }

        let mut split_line = line_slice.split_ascii_whitespace();
        match split_line.next() {
            Some(string) => match string {
                "$date" => {
                    self.line.clear();
                    while !self.line.contains("$end") {
                        self.read_line_noclear();
                    }
                    Some(LineInfo::DateInfo(self.line.clone()))
                }
                "$version" => {
                    self.line.clear();
                    while !self.line.contains("$end") {
                        self.read_line_noclear();
                    }
                    Some(LineInfo::VersionInfo(self.line.clone()))
                }
                "$timescale" => {
                    self.line.clear();
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
                "$enddefinitions" => Some(LineInfo::EndDefinitions),
                _ => Self::unrecognized_symbol(string, self.lineno),
            },
            None => {
                unreachable!("WARNING: Empty line passed filtering! This should not have happened!")
            }
        }
    }
}
