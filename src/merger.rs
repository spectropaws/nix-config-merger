// merger.rs
//! Logic to merge two maps of key-value pairs into nested Nix attribute blocks.

use std::collections::HashMap;
use rnix::ast::Expr;

/// A node in the nested attribute tree: either a leaf value or a subtree.
#[derive(Clone)]
enum Node {
    Leaf(Expr),
    Tree(HashMap<String, Node>),
}

/// Inserts a leaf `value` into the `tree` at the given `path` (sequence of keys).
fn insert_node(tree: &mut HashMap<String, Node>, path: &[String], value: Expr) {
    if path.len() == 1 {
        tree.insert(path[0].clone(), Node::Leaf(value));
    } else {
        let head = &path[0];
        let sub = tree.entry(head.clone())
            .or_insert_with(|| Node::Tree(HashMap::new()));
        match sub {
            Node::Tree(m) => insert_node(m, &path[1..], value),
            Node::Leaf(_) => {
                let mut new_map = HashMap::new();
                insert_node(&mut new_map, &path[1..], value);
                *sub = Node::Tree(new_map);
            }
        }
    }
}

/// Merges two attribute trees, with `new` overriding or extending `old`.
fn merge_node(old: Node, new: Node) -> Node {
    match (old, new) {
        (Node::Tree(mut om), Node::Tree(nm)) => {
            for (k, nv) in nm {
                let merged = if let Some(ov) = om.remove(&k) {
                    merge_node(ov, nv)
                } else {
                    nv
                };
                om.insert(k, merged);
            }
            Node::Tree(om)
        }
        (_, nv) => nv,
    }
}

/// Builds a nested tree from a flat key→Expr map.
fn build_tree(flat: HashMap<String, Expr>) -> HashMap<String, Node> {
    let mut tree = HashMap::new();
    for (k, v) in flat {
        let path: Vec<String> = k.split('.').map(String::from).collect();
        insert_node(&mut tree, &path, v);
    }
    tree
}

/// Merges two flat maps into a nested, merged tree and serializes it.
pub fn merge_maps(
    old_map: HashMap<String, Expr>,
    new_map: HashMap<String, Expr>,
) -> String {
    let old_tree = build_tree(old_map);
    let new_tree = build_tree(new_map);
    let mut full = old_tree;
    for (k, nv) in new_tree {
        let merged = if let Some(ov) = full.remove(&k) {
            merge_node(ov, nv)
        } else {
            nv
        };
        full.insert(k, merged);
    }


    /// Serialize with improved newlines for prettiness
    fn serialize(tree: &HashMap<String, Node>, indent: usize) -> String {
    let mut out = String::new();
    let pad = "  ".repeat(indent);

    // 1. Collect entries into a Vec so we can see "what comes next"
    let entries: Vec<_> = tree.iter().collect();
    // (Optional) Sort to stabilize output; remove if you want insertion order
    // entries.sort_by_key(|(k, _)| *k);

    let len = entries.len();
    for (i, (k, node)) in entries.into_iter().enumerate() {
        match node {
            Node::Leaf(expr) => {
                out.push_str(&format!("{}{} = {};\n", pad, k, expr));
            }
            Node::Tree(sub) => {
                // 2-level shortcut (e.g. openssh.authorizedKeys.keys = …)
                if sub.len() == 1 {
                    let (child_k, child_node) = sub.iter().next().unwrap();
                    if let Node::Leaf(expr) = child_node {
                        out.push_str(&format!("{}{}.{} = {};\n",
                            pad, k, child_k, expr));
                        continue;
                    }
                }

                // Normal block
                out.push_str(&format!("{}{} = {{\n", pad, k));
                out.push_str(&serialize(sub, indent + 1));
                out.push_str(&format!("{}}};\n", pad));

                //  • If this isn’t the last entry in *this* block, add one blank line
                if i + 1 < len {
                    out.push_str("\n");
                }
            }
        }
    }

    out
}

    let mut out = String::from("{

");
    out.push_str(&serialize(&full, 1));
    out.push_str("}
");
    out
}
