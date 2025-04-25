// src/main.rs
// A CLI tool to merge two NixOS configuration files using our rnix-attempt library.

use std::error::Error;
use std::{env, fs, process};
use nix_config_merger::merge_configs;

fn main() -> Result<(), Box<dyn Error>> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();  
    // Ensure exactly two input files are provided
    if args.len() != 3 {
        eprintln!("Usage: {} <old.nix> <new.nix>", args[0]);
        process::exit(1);
    }

    // Read the contents of both Nix config files
    let old_nix = fs::read_to_string(&args[1])?;  
    let new_nix = fs::read_to_string(&args[2])?;

    // Merge using the library function
    let merged = merge_configs(&old_nix, &new_nix)?;

    // Print merged config to stdout
    println!("{}", merged);

    Ok(())
}
