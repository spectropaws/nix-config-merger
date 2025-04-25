// path_normalizer.rs
//! Converts AST attribute paths into canonical dot-separated strings.

use rnix::ast::Attrpath;

/// Converts an `Attrpath` node into a dot-separated key string.
pub fn normalize_path(ap: &Attrpath) -> String {
    ap.attrs().map(|a| a.to_string()).collect::<Vec<_>>().join(".")
}

