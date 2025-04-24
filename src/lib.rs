// use pyo3::prelude::*;
// use pyo3::wrap_pyfunction;

use rnix::ast::{Root, Expr, Entry, Attr, HasEntry, AstNode};     
use rnix::{SyntaxNode, SyntaxKind, TextRange};                  
use rowan::ast::AstNode as _;                                  
use std::collections::HashMap;
use std::fs;


/// Flatten a NODE_ATTRPATH like `services.nginx.enable` -> `"services.nginx.enable"`.
fn flatten_attrpath(node: &SyntaxNode) -> Option<String> {
    if node.kind() != SyntaxKind::NODE_ATTRPATH {
        return None;
    }
    let segments: Vec<_> = node
        .children()
        .filter_map(|child| {
            if child.kind() == SyntaxKind::TOKEN_IDENT {
                Some(child.text().to_string())
            } else {
                None
            }
        })
        .collect();
    if segments.is_empty() { None } else { Some(segments.join(".")) }
}

/*
/// Extract `key = value;` nodes into a map.
fn extract_assignments(root: &SyntaxNode) -> HashMap<String, SyntaxNode> {
    let mut map = HashMap::new();
    for node in root.descendants() {
        if node.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
            // The first child is the Attrpath, the whole node is the binding
            if let Some(path_node) = node.children().next() {
                if let Some(key) = flatten_attrpath(&path_node) {
                    map.insert(key, node.clone());
                }
            }
        }
    }
    map
}
*/


/// Scan `source` for every `foo.bar = _;` binding.
/// Returns a map `key -> (span, full_text)`.
fn extract_assignments(source: &str)
    -> HashMap<String, (TextRange, String)>
{
    let mut map = HashMap::new();

    // 1) Parse into the typed AST (never fails here)
    let root = Root::parse(source)
        .ok()
        .expect("Failed to parse Nix source");         

    // 2) Get the top-level expression
    let mut expr = root.expr()
        .expect("No top-level expression found");

    // 3) If it's a lambda (`{…}: {...}`), unwrap its body
    if let Expr::Lambda(lambda) = expr {
        expr = lambda.body()
            .expect("Lambda missing body");
    }

    // 4) Only proceed if it's an attribute set
    if let Expr::AttrSet(attr_set) = expr {
        // 5) Iterate every `foo.bar = value;` entry
        for entry in attr_set.entries() {            // via HasEntry :contentReference[oaicite:4]{index=4}
            if let Entry::AttrpathValue(av) = entry { // the `key = value;` variant
                // Flatten path segments to "foo.bar"
                let key = av.attrpath().unwrap()
                    .attrs()
                    .filter_map(|attr| {
                        if let Attr::Ident(ident) = attr {
                            ident.ident_token()
                                 .map(|tok| tok.text().to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(".");

                // Capture its range + exact snippet
                let node  = av.syntax();
                let range = node.text_range();       // from rowan::SyntaxNode :contentReference[oaicite:5]{index=5}
                let text  = node.text().to_string();

                map.insert(key, (range, text));
            }
        }
    }

    map
}


/*
// #[pyfunction]
pub fn merge_nix_configs(old_path: &str, new_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let old_src = fs::read_to_string(old_path)?;
    let new_src = fs::read_to_string(new_path)?;

    // Parse both configs
    let old_root = Root::parse(&old_src).syntax();
    let new_root = Root::parse(&new_src).syntax();

    // debug 
    let old_map = extract_assignments(&old_root);
    eprintln!("Old config keys ({}):", old_map.len());
    for k in old_map.keys() {
        eprintln!("  - {}", k);
    }

    let new_map = extract_assignments(&new_root);
    eprintln!("New config keys ({}):", new_map.len());
    for k in new_map.keys() {
        eprintln!("  - {}", k);
    }
    // debug end

    let mut old_map = extract_assignments(&old_root);
    let new_map = extract_assignments(&new_root);

    let mut result = old_src.clone();
    for (key, new_node) in new_map {
        if let Some(old_node) = old_map.remove(&key) {
            let range = old_node.text_range();
            let start: usize = range.start().into();
            let end:   usize = range.end().into();
            result.replace_range(start..end, &new_node.text().to_string());
        } else {
            result.push_str("\n");
            result.push_str(&new_node.text().to_string());
        }
    }

    Ok(result)
}
*/



/// Merges `new.nix` into `old.nix`, updating changed keys and appending missing ones.
pub fn merge_nix_configs(old_path: &str, new_path: &str)
    -> Result<String, Box<dyn std::error::Error>>
{
    let old_src = fs::read_to_string(old_path)?;
    let new_src = fs::read_to_string(new_path)?;

    let old_map = extract_assignments(&old_src);
    let new_map = extract_assignments(&new_src);

    // (Optional) debug:
    eprintln!("old keys: {:?}", old_map.keys());
    eprintln!("new keys: {:?}", new_map.keys());

    let mut result = old_src.clone();
    let mut replacements = Vec::new();
    let mut additions = Vec::new();

    // 1) Update existing keys, collect new ones
    for (key, (_new_range, new_txt)) in &new_map {
        if let Some((old_range, old_txt)) = old_map.get(key) {
            if old_txt != new_txt {
                let start: usize = old_range.start().into();
                let end: usize = old_range.end().into();
                replacements.push((start, end, new_txt.clone()));
            }
        } else {
            additions.push(new_txt.clone());
        }
    }

    // 2) Apply replacements in reverse order
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    for (s, e, txt) in replacements {
        result.replace_range(s..e, &txt);
    }

    // 3) Insert additions before the last closing brace in the file
    if !additions.is_empty() {
        if let Some(insert_pos) = result.rfind('}') {
            let joined = additions
                .into_iter()
                .map(|s| format!("\n{}", s))
                .collect::<String>();
            result.insert_str(insert_pos, &joined);
        } else {
            // Fallback: no closing brace found, just append
            eprintln!("Warning: Couldn't find closing '}}' in old config, appending at the end.");
            for add in additions {
                result.push_str("\n");
                result.push_str(&add);
            }
        }
    }

    Ok(result)
}


/* #[pymodule]
fn nixos_configurator(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(merge_nix_configs, m)?)?;
    Ok(())
}
*/
