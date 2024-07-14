use std::env;

use vcd_statistical_analysis::{self, perform_analysis};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        panic!("Expected usage: vcd <infile> <outfile>")
    }
    perform_analysis(&args[1], &args[2]);
}
