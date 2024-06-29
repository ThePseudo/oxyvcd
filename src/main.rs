use std::env;

use vcd_statistical_analysis::{self, perform_analysis};

fn main() {
    let args: Vec<String> = env::args().collect();
    perform_analysis(&args[1]);
}
