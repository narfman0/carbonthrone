//! Lightweight Yarn dialog interpreter for the CLI frontend.
//!
//! Parses and executes the same `.yarn` scripts used by the GUI's
//! `bevy_yarnspinner` plugin, so the CLI can present full interactive dialog
//! without depending on Bevy or the GUI crate.

use std::collections::HashMap;
use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, ClearType},
};

// Embed all five loop scripts at compile time.
const LOOP1: &str = include_str!("../../gui/assets/dialogue/loop1.yarn");
const LOOP2: &str = include_str!("../../gui/assets/dialogue/loop2.yarn");
const LOOP3: &str = include_str!("../../gui/assets/dialogue/loop3.yarn");
const LOOP4: &str = include_str!("../../gui/assets/dialogue/loop4.yarn");
const LOOP5: &str = include_str!("../../gui/assets/dialogue/loop5.yarn");

fn yarn_for_loop(n: u32) -> &'static str {
    match n {
        1 => LOOP1,
        2 => LOOP2,
        3 => LOOP3,
        4 => LOOP4,
        5 => LOOP5,
        _ => "",
    }
}

// ── Variable values ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum Val {
    Bool(bool),
    Str(String),
}

impl Val {
    fn as_bool(&self) -> bool {
        match self {
            Val::Bool(b) => *b,
            Val::Str(s) => !s.is_empty(),
        }
    }
}

// ── Runner ────────────────────────────────────────────────────────────────────

pub struct YarnRunner {
    nodes: HashMap<String, Vec<String>>,
    vars: HashMap<String, Val>,
    /// Flags that were already set at init — used to compute the delta.
    initial_flags: std::collections::HashSet<String>,
    /// Dialog lines accumulated so far (used as context when showing choices).
    history: Vec<(String, String)>,
}

impl YarnRunner {
    /// Build a runner seeded with the current game state.
    pub fn new(
        loop_number: u32,
        flags: &std::collections::HashSet<String>,
        companion: Option<&str>,
    ) -> Self {
        let text = yarn_for_loop(loop_number);
        let nodes = parse_nodes(text);

        let mut vars: HashMap<String, Val> = HashMap::new();
        for flag in flags {
            vars.insert(flag.clone(), Val::Bool(true));
        }
        if let Some(c) = companion {
            vars.insert("companion".to_string(), Val::Str(c.to_string()));
        }

        Self {
            nodes,
            vars,
            initial_flags: flags.clone(),
            history: Vec::new(),
        }
    }

    /// `true` when the named node exists in the compiled script.
    pub fn has_node(&self, name: &str) -> bool {
        self.nodes.contains_key(name)
    }

    /// Execute `node_name`, interacting with the user via `stdout`.
    pub fn run(&mut self, node_name: &str, stdout: &mut io::Stdout) {
        if let Some(lines) = self.nodes.get(node_name).cloned() {
            self.exec(&lines, stdout);
        }
    }

    /// Return every boolean flag set to `true` during this dialog that was not
    /// already set at construction time.  Pass this to `session.end_dialog()`.
    pub fn new_flags(&self) -> Vec<String> {
        self.vars
            .iter()
            .filter_map(|(k, v)| {
                if v.as_bool() && !self.initial_flags.contains(k) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    // ── Private execution engine ─────────────────────────────────────────────

    /// Execute a slice of (raw) Yarn lines.  Returns `true` when a `<<jump>>`
    /// was encountered so the caller knows to stop executing the current node.
    fn exec(&mut self, lines: &[String], stdout: &mut io::Stdout) -> bool {
        let mut i = 0;
        while i < lines.len() {
            let raw = &lines[i];
            let t = raw.trim();

            if t.is_empty() || t.starts_with("//") {
                i += 1;
                continue;
            }

            // ── Commands ──────────────────────────────────────────────────
            if t.starts_with("<<") {
                if let Some(target) = parse_jump(t) {
                    self.run(&target, stdout);
                    return true; // jump → stop current node
                } else if let Some((var, val)) = parse_set(t) {
                    self.do_set(var, val);
                } else if t.starts_with("<<if ") {
                    let (end_i, jumped) = self.exec_if_block(lines, i, stdout);
                    i = end_i;
                    if jumped {
                        return true;
                    }
                    continue;
                }
                // <<endif>> / <<else>> / <<elseif>> are consumed by exec_if_block
                i += 1;
                continue;
            }

            // ── Choices ───────────────────────────────────────────────────
            if t.starts_with("-> ") {
                let (choices, end_i) = collect_choices(lines, i);
                let texts: Vec<&str> = choices.iter().map(|(t, _)| t.as_str()).collect();
                let sel = self.prompt_choice(stdout, &texts);
                let body = choices[sel].1.clone();
                let jumped = self.exec(&body, stdout);
                i = end_i;
                if jumped {
                    return true;
                }
                continue;
            }

            // ── Dialog line ───────────────────────────────────────────────
            if let Some((speaker, text)) = parse_dialog(t) {
                self.show_line(stdout, &speaker, &text);
            }

            i += 1;
        }
        false
    }

    fn do_set(&mut self, var: String, val: Val) {
        self.vars.insert(var, val);
    }

    /// Execute an `<<if>> … <<endif>>` block starting at `if_idx`.
    /// Returns `(index_after_endif, jumped)`.
    fn exec_if_block(
        &mut self,
        lines: &[String],
        if_idx: usize,
        stdout: &mut io::Stdout,
    ) -> (usize, bool) {
        let (branches, end_idx) = split_if_block(lines, if_idx);
        for (cond, body) in &branches {
            let active = cond
                .as_deref()
                .map(|c| eval_expr(c, &self.vars))
                .unwrap_or(true); // <<else>> branch
            if active {
                let jumped = self.exec(body, stdout);
                return (end_idx, jumped);
            }
        }
        (end_idx, false)
    }

    // ── Terminal UI ──────────────────────────────────────────────────────────

    fn show_line(&mut self, stdout: &mut io::Stdout, speaker: &str, text: &str) {
        self.history.push((speaker.to_string(), text.to_string()));
        render_dialog_screen(stdout, &self.history, &[], false);
        wait_key();
    }

    fn prompt_choice(&mut self, stdout: &mut io::Stdout, options: &[&str]) -> usize {
        render_dialog_screen(stdout, &self.history, options, true);
        loop {
            let Ok(ev) = event::read() else { continue };
            let Event::Key(k) = ev else { continue };
            if k.kind != KeyEventKind::Press {
                continue;
            }
            if let KeyCode::Char(c) = k.code {
                if let Some(n) = c.to_digit(10) {
                    let idx = n as usize;
                    if idx >= 1 && idx <= options.len() {
                        return idx - 1;
                    }
                }
            }
        }
    }
}

// ── Yarn parser helpers ───────────────────────────────────────────────────────

/// Split a Yarn source into a map of `title → body_lines`.
fn parse_nodes(text: &str) -> HashMap<String, Vec<String>> {
    let mut nodes: HashMap<String, Vec<String>> = HashMap::new();
    let mut title: Option<String> = None;
    let mut body: Vec<String> = Vec::new();
    let mut in_body = false;

    for line in text.lines() {
        let t = line.trim();
        if !in_body {
            if let Some(name) = t.strip_prefix("title: ") {
                title = Some(name.trim().to_string());
            } else if t == "---" {
                in_body = true;
                body.clear();
            }
        } else if t == "===" {
            if let Some(name) = title.take() {
                nodes.insert(name, body.clone());
            }
            body.clear();
            in_body = false;
        } else {
            body.push(line.to_string());
        }
    }
    nodes
}

fn parse_jump(t: &str) -> Option<String> {
    let inner = t.strip_prefix("<<jump ")?.strip_suffix(">>")?;
    Some(inner.trim().to_string())
}

fn parse_set(t: &str) -> Option<(String, Val)> {
    let inner = t.strip_prefix("<<set ")?.strip_suffix(">>")?;
    let (lhs, rhs) = inner.split_once(" = ")?;
    let var = lhs.trim().trim_start_matches('$').to_string();
    let rhs = rhs.trim();
    let val = if rhs == "true" {
        Val::Bool(true)
    } else if rhs == "false" {
        Val::Bool(false)
    } else if rhs.starts_with('"') && rhs.ends_with('"') {
        Val::Str(rhs[1..rhs.len() - 1].to_string())
    } else {
        Val::Bool(true)
    };
    Some((var, val))
}

/// Split an `<<if>> … <<endif>>` block (starting at `if_idx`) into branches.
///
/// Returns `(Vec<(Option<condition_str>, body_lines)>, index_after_endif)`.
fn split_if_block(lines: &[String], if_idx: usize) -> (Vec<(Option<String>, Vec<String>)>, usize) {
    let mut branches: Vec<(Option<String>, Vec<String>)> = Vec::new();

    // Extract condition from the opening <<if cond>> line
    let first = lines[if_idx].trim();
    let opening_cond = first
        .strip_prefix("<<if ")
        .and_then(|s| s.strip_suffix(">>"))
        .map(|s| s.trim().to_string());
    let mut cur_cond: Option<String> = opening_cond;
    let mut cur_body: Vec<String> = Vec::new();
    let mut depth: usize = 1;
    let mut i = if_idx + 1;

    while i < lines.len() {
        let t = lines[i].trim();

        if t.starts_with("<<if ") {
            depth += 1;
            cur_body.push(lines[i].clone());
        } else if t == "<<endif>>" {
            depth -= 1;
            if depth == 0 {
                branches.push((cur_cond, cur_body));
                return (branches, i + 1);
            } else {
                cur_body.push(lines[i].clone());
            }
        } else if depth == 1 && t == "<<else>>" {
            branches.push((cur_cond, cur_body));
            cur_cond = None;
            cur_body = Vec::new();
        } else if depth == 1 && t.starts_with("<<elseif ") {
            branches.push((cur_cond, cur_body));
            cur_cond = t
                .strip_prefix("<<elseif ")
                .and_then(|s| s.strip_suffix(">>"))
                .map(|s| s.trim().to_string());
            cur_body = Vec::new();
        } else {
            cur_body.push(lines[i].clone());
        }

        i += 1;
    }

    // Reached end of node without <<endif>> — treat as closed
    branches.push((cur_cond, cur_body));
    (branches, i)
}

/// Collect consecutive `-> choice` options starting at `start`.
///
/// Returns `(Vec<(text, body_lines)>, index_after_last_choice_body)`.
fn collect_choices(lines: &[String], start: usize) -> (Vec<(String, Vec<String>)>, usize) {
    let mut choices: Vec<(String, Vec<String>)> = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let t = lines[i].trim();
        if let Some(choice_text) = t.strip_prefix("-> ") {
            choices.push((choice_text.to_string(), Vec::new()));
            i += 1;
            // Collect indented body lines
            while i < lines.len() && (lines[i].starts_with("    ") || lines[i].starts_with('\t')) {
                if let Some(last) = choices.last_mut() {
                    last.1.push(lines[i].trim().to_string());
                }
                i += 1;
            }
        } else {
            break;
        }
    }

    (choices, i)
}

/// Parse `"speaker: text"` dialog lines.  Returns `None` for commands or lines
/// that don't match the pattern.
fn parse_dialog(t: &str) -> Option<(String, String)> {
    if t.starts_with("<<") || t.starts_with("-> ") {
        return None;
    }
    let colon = t.find(':')?;
    let speaker = &t[..colon];
    // Speaker names are single words (no spaces, no angle brackets)
    if speaker.contains(' ') || speaker.contains('<') {
        return None;
    }
    let text = t[colon + 1..].trim();
    Some((speaker.to_string(), text.to_string()))
}

// ── Expression evaluator ──────────────────────────────────────────────────────

fn eval_expr(expr: &str, vars: &HashMap<String, Val>) -> bool {
    let expr = expr.trim();

    // Logical AND — split on the last (rightmost) " and " so that
    // "$a == \"x\" and $b" is parsed as (== check) AND ($b).
    if let Some(pos) = rfind_binary_op(expr, " and ") {
        return eval_expr(&expr[..pos], vars) && eval_expr(&expr[pos + 5..], vars);
    }

    // Logical OR
    if let Some(pos) = rfind_binary_op(expr, " or ") {
        return eval_expr(&expr[..pos], vars) || eval_expr(&expr[pos + 4..], vars);
    }

    // Negation
    if let Some(inner) = expr.strip_prefix("not ") {
        return !eval_expr(inner.trim(), vars);
    }

    // Equality: $var == "string"
    if let Some(eq) = expr.find(" == ") {
        let lhs = expr[..eq].trim().trim_start_matches('$');
        let rhs = expr[eq + 4..].trim().trim_matches('"');
        return match vars.get(lhs) {
            Some(Val::Str(s)) => s == rhs,
            Some(Val::Bool(true)) => rhs == "true",
            _ => rhs == "false" || rhs.is_empty(),
        };
    }

    // Single variable
    let var = expr.trim_start_matches('$');
    vars.get(var).map(|v| v.as_bool()).unwrap_or(false)
}

/// Find the position of `op` in `s` scanning from right, skipping occurrences
/// inside double-quoted strings.
fn rfind_binary_op(s: &str, op: &str) -> Option<usize> {
    // Simple scan: find all occurrences and return the last one that is not
    // inside quotes (quotes only appear in == comparisons which come before
    // " and "/" or " in practice).
    let mut last = None;
    let mut start = 0;
    while let Some(pos) = s[start..].find(op) {
        let abs = start + pos;
        last = Some(abs);
        start = abs + op.len();
    }
    last
}

// ── Terminal rendering ────────────────────────────────────────────────────────

const WIDTH: usize = 58;

/// Render a full-screen dialog frame.
///
/// `history` – recent dialog lines shown as context (last few).
/// `choices`  – when non-empty, display as numbered options.
/// `waiting_for_choice` – controls the footer hint.
fn render_dialog_screen(
    stdout: &mut io::Stdout,
    history: &[(String, String)],
    choices: &[&str],
    waiting_for_choice: bool,
) {
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )
    .unwrap();

    let bar = "=".repeat(WIDTH);
    write!(stdout, "{bar}\r\n").unwrap();
    write!(
        stdout,
        "{:^width$}\r\n",
        "C A R B O N T H R O N E",
        width = WIDTH
    )
    .unwrap();
    write!(stdout, "{bar}\r\n\r\n").unwrap();

    // Show the last few lines of dialog history as context
    let context_lines = 5usize;
    let start = history.len().saturating_sub(context_lines);
    for (speaker, text) in &history[start..] {
        let label = format!("[{}]", speaker.to_uppercase());
        // Word-wrap text to fit WIDTH - 4 (indent)
        for wrapped in wrap_text(text, WIDTH.saturating_sub(label.len() + 3)) {
            write!(stdout, "  {label} {wrapped}\r\n").unwrap();
        }
    }

    write!(stdout, "\r\n").unwrap();
    write!(stdout, "  {}\r\n", "-".repeat(WIDTH - 2)).unwrap();

    if !choices.is_empty() {
        write!(stdout, "\r\n").unwrap();
        for (i, choice) in choices.iter().enumerate() {
            write!(stdout, "  [{}] {}\r\n", i + 1, choice).unwrap();
        }
        write!(stdout, "\r\n").unwrap();
    }

    write!(stdout, "{bar}\r\n").unwrap();
    if waiting_for_choice {
        write!(stdout, "  Press 1-{} to choose\r\n", choices.len()).unwrap();
    } else {
        write!(stdout, "  [any key] next\r\n").unwrap();
    }
    write!(stdout, "{bar}\r\n").unwrap();

    stdout.flush().unwrap();
}

/// Very simple word-wrapper — splits on spaces to keep lines under `max_width`.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || text.len() <= max_width {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current.clone());
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn wait_key() {
    loop {
        let Ok(ev) = event::read() else { continue };
        let Event::Key(k) = ev else { continue };
        if k.kind == KeyEventKind::Press {
            return;
        }
    }
}
