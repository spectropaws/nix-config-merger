// ast_utils.rs
//! Utilities for parsing Nix code, unwrapping optional lambdas, and extracting key-value pairs.

use crate::path_normalizer;
use rnix::Root;
use rnix::ast::{AttrSet, Entry, Expr, HasEntry};
use std::{collections::HashMap, error::Error};

/// Parses Nix code into an AST `Root`, checking for parse errors.
fn parse_nix(code: &str) -> Result<Root, Box<dyn Error>> {
    let parsed = Root::parse(code);
    if !parsed.errors().is_empty() {
        return Err(format!("Parse errors: {:?}", parsed.errors()).into());
    }
    // Safe unwrap: errors list is empty
    Ok(parsed.ok().unwrap())
}

/// Recursively unwraps nested lambdas to find the inner attribute set node.
fn unwrap_to_attrset(expr: Expr) -> Result<AttrSet, Box<dyn Error>> {
    match expr {
        Expr::AttrSet(set) => Ok(set),
        Expr::Lambda(lambda) => {
            let body = lambda.body().ok_or("Lambda has no body")?;
            unwrap_to_attrset(body)
        }
        _ => Err("Expected attribute set or lambda wrapping one".into()),
    }
}

/// Extracts a flattened map of `key -> Expr` from Nix code,
/// returning an optional lambda prefix and the map.

pub fn extract_kv_pairs(
    nix_code: &str,
) -> Result<(Option<String>, HashMap<String, Expr>), Box<dyn Error>> {
    let root = parse_nix(nix_code)?;
    let expr = root.expr().ok_or("Root is not an Expr")?;

    let (prefix, attr_expr) = match &expr {
        Expr::Lambda(lambda) => {
            let lambda_str = lambda.to_string();
            let body = lambda.body().ok_or("Lambda has no body")?;
            let body_str = body.to_string();
            let lambda_prefix = lambda_str.strip_suffix(&body_str).unwrap_or("").to_string();
            (Some(lambda_prefix), body.clone())
        }
        _ => (None, expr.clone()),
    };

    let set = unwrap_to_attrset(attr_expr)?;

    // Flatten into map
    let mut map = HashMap::new();
    recurse_extract(&set, String::new(), &mut map);
    Ok((prefix, map))
}

/// Recursively traverses an `AttrSet` to flatten nested keys into the map.
fn recurse_extract(set: &AttrSet, prefix: String, map: &mut HashMap<String, Expr>) {
    for entry in set.entries() {
        if let Entry::AttrpathValue(av) = entry {
            let ap = av.attrpath().unwrap();
            let key = path_normalizer::normalize_path(&ap);
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };

            if let Some(val) = av.value() {
                if let Expr::AttrSet(inner) = &val {
                    recurse_extract(inner, full_key, map);
                } else {
                    map.insert(full_key, val.clone());
                }
            }
        }
    }
}
