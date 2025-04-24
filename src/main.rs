use std::env;
use std::fs;

use nix_merger::merge_nix_configs; 

fn main() {
    // Collect command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <old_file> <new_file>", args[0]);
        std::process::exit(1);
    }

    let old_path = &args[1];
    let new_path = &args[2];

    match merge_nix_configs(old_path, new_path) {
        Ok(merged) => {
            println!("{}", merged);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

