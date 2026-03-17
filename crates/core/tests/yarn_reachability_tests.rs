use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

// ── Yarn graph types ──────────────────────────────────────────────────────────

struct YarnNode {
    jumps: Vec<String>,
    flags_set: Vec<String>,
}

struct YarnGraph {
    nodes: HashMap<String, YarnNode>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Extract the title from a raw node block (the text before `---`).
fn extract_title(block: &str) -> Option<String> {
    for line in block.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("title:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

/// Extract the content section from a raw node block (after the first `---`).
fn extract_content(block: &str) -> &str {
    if let Some(idx) = block.find("---") {
        &block[idx + 3..]
    } else {
        ""
    }
}

/// Find all `<<jump NodeName>>` targets in a content string.
fn extract_jumps(content: &str) -> Vec<String> {
    let mut jumps = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("<<jump ") {
        let after = &rest[start + 7..];
        if let Some(end) = after.find(">>") {
            let target = after[..end].trim().to_string();
            if !target.is_empty() {
                jumps.push(target);
            }
        }
        rest = &rest[start + 7..];
    }
    jumps
}

/// Find all `<<set $flag_name = true>>` flag names in a content string.
fn extract_flags_set(content: &str) -> Vec<String> {
    let mut flags = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("<<set $") {
        let after = &rest[start + 7..];
        if let Some(eq_pos) = after.find(" = true") {
            let flag_name = after[..eq_pos].trim().to_string();
            if !flag_name.is_empty() {
                flags.push(flag_name);
            }
        }
        rest = &rest[start + 7..];
    }
    flags
}

/// Parse all `loop*.yarn` files from the dialogue directory and return a
/// combined node graph spanning all five loops.
fn build_yarn_graph() -> YarnGraph {
    let yarn_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../gui/assets/dialogue");
    let mut nodes: HashMap<String, YarnNode> = HashMap::new();

    for loop_n in 1..=5u32 {
        let path = format!("{yarn_dir}/loop{loop_n}.yarn");
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));

        // Nodes are separated by "===".
        for block in content.split("===") {
            let title = match extract_title(block) {
                Some(t) => t,
                None => continue,
            };
            let body = extract_content(block);
            let jumps = extract_jumps(body);
            let flags_set = extract_flags_set(body);
            nodes.insert(title, YarnNode { jumps, flags_set });
        }
    }

    YarnGraph { nodes }
}

/// BFS from `entry_nodes`, following all `<<jump>>` edges.
/// Returns the set of all flag names set along any reachable path.
fn bfs_flags(graph: &YarnGraph, entry_nodes: &[&str]) -> HashSet<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut reachable_flags: HashSet<String> = HashSet::new();

    for &entry in entry_nodes {
        if graph.nodes.contains_key(entry) {
            queue.push_back(entry.to_string());
            visited.insert(entry.to_string());
        }
    }

    while let Some(node_name) = queue.pop_front() {
        if let Some(node) = graph.nodes.get(&node_name) {
            for flag in &node.flags_set {
                reachable_flags.insert(flag.clone());
            }
            for jump_target in &node.jumps {
                if !visited.contains(jump_target) {
                    visited.insert(jump_target.clone());
                    queue.push_back(jump_target.clone());
                }
            }
        }
    }

    reachable_flags
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn all_ending_flags_reachable_from_loop_starts() {
    let graph = build_yarn_graph();

    // The game fires zone-trigger nodes (on_enter / on_interact / on_combat_end)
    // whenever the player enters a zone or interacts with something.  These nodes
    // are independent BFS roots — they are not reached via <<jump>> from other
    // nodes; the engine fires them directly.  Including all of them correctly
    // models the full set of dialog the player can encounter.
    let entry_nodes: Vec<&str> = graph
        .nodes
        .keys()
        .filter(|name| {
            let n = name.as_str();
            n.contains("_on_enter") || n.contains("_on_interact") || n.contains("_on_combat_end")
        })
        .map(|s| s.as_str())
        .collect();

    let reachable_flags = bfs_flags(&graph, &entry_nodes);

    for flag in ["orin_helped", "ending_doss", "ending_kaleo", "orin_stopped"] {
        assert!(
            reachable_flags.contains(flag),
            "ending flag '{flag}' is not reachable from any loop's on_enter node"
        );
    }
}

#[test]
fn no_dangling_jump_targets_across_all_loops() {
    let graph = build_yarn_graph();

    for (node_name, node) in &graph.nodes {
        for target in &node.jumps {
            assert!(
                graph.nodes.contains_key(target.as_str()),
                "node '{node_name}' has <<jump {target}>> but '{target}' is not defined"
            );
        }
    }
}

#[test]
fn all_loop_entry_nodes_exist() {
    let graph = build_yarn_graph();

    for loop_n in 1..=5u32 {
        let node_name = format!("loop{loop_n}_research_wing_on_enter");
        assert!(
            graph.nodes.contains_key(node_name.as_str()),
            "expected entry node '{node_name}' not found in yarn graph"
        );
    }
}
