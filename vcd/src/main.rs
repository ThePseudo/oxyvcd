use clap::Parser;
use logger::Log;
use std::io::stdout;
use vcd_statistical_analysis::{self, perform_analysis_and_save, Configuration};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file path
    #[arg(short, long)]
    in_file: String,
    /// Output file path
    #[arg(short, long)]
    out_file: String,
    /// Separator for changes.
    #[arg(short, long, default_value_t = '<')]
    separator: char,
}

fn main() {
    let args = Args::parse();
    let c = Configuration {
        in_file: args.in_file,
        out_file: args.out_file,
        separator: args.separator,
        use_spinner: true,
    };
    Log::add(Box::new(stdout().lock()));
    if let Err(e) = perform_analysis_and_save(c) {
        Log::write(logger::Priority::Error, &e.to_string());
    }
}
