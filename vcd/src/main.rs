use clap::Parser;
use vcd_statistical_analysis::{self, perform_analysis, Configuration};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    in_file: String,
    #[arg(short, long)]
    out_file: String,
    #[arg(short, long, default_value_t = '<')]
    separator: char,
}

fn main() {
    let args = Args::parse();
    let c = Configuration {
        in_file: args.in_file,
        out_file: args.out_file,
        separator: args.separator,
    };
    perform_analysis(c);
}
