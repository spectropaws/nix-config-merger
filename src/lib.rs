// lib.rs
//! A Rust library for merging two NixOS configuration files (old.nix and new.nix)
//! using rnix-parser (v0.12.0) and rowan (v0.15.x), with support for top-level lambdas.

pub mod ast_utils;
pub mod path_normalizer;
pub mod merger;

use std::error::Error;

/// Merges two Nix configurations by updating or inserting key-value pairs
/// from `new_nix` into `old_nix`, preserving nested attribute blocks
/// and handling optional top-level lambdas.
///
/// # Errors
/// Returns an error if parsing of either configuration fails.
pub fn merge_configs(old_nix: &str, new_nix: &str) -> Result<String, Box<dyn Error>> {
    let (old_prefix, old_map) = ast_utils::extract_kv_pairs(old_nix)?;
    let (new_prefix, new_map) = ast_utils::extract_kv_pairs(new_nix)?;

    // Perform nested merge
    let merged_body = merger::merge_maps(old_map, new_map);

    // Preserve old lambda if present, else use new, else none
    let prefix = old_prefix.or(new_prefix);
    let result = if let Some(p) = prefix {
        format!("{}{}", p, merged_body)
    } else {
        merged_body
    };
    Ok(result)
}
