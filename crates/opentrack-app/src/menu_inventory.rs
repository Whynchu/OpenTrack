//! Generated main-menu and shortcut inventory.
//!
//! The menu builder is ordinary Rust code, so this module scans the source text
//! of `show_menu` and `show_empty_project_menu` and records every leaf it can
//! reach: literal rows, state-gated rows and the simple `for` families the
//! builder expands. It is deliberately a source register, not runtime proof:
//! a disabled row is still disabled, and a dispatch arm is not evidence that a
//! command reached working behaviour.
//!
//! The generated report is checked in at
//! `docs/reviews/generated-source-inventory.md`. Regenerate it after changing
//! the menu or the bundled keymap:
//!
//! ```text
//! $env:UPDATE_SOURCE_INVENTORY = "1"
//! cargo test -p opentrack generated_source_inventory -- --nocapture
//! ```
//!
//! The test then fails if the committed report is stale.

use opentrack_core::commands::{ArrowKey, CommandCatalog, KeyCode, KeymapProfile};
use std::collections::{BTreeMap, BTreeSet, HashMap};

const REPORT_PATH: &str = "../../docs/reviews/generated-source-inventory.md";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    /// The row's availability argument is the literal `true` (or a toggle that
    /// is always clickable), so it is offered whenever the menu is built.
    Available,
    /// The row's availability comes from project or selection state.
    StateDependent,
    /// The row is built with literal `false`, `add_enabled(false, ...)` or is
    /// listed by `disabled_menu_commands`.
    Unavailable,
    /// A caption or explanation, not a command.
    Informational,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::StateDependent => "state-dependent",
            Self::Unavailable => "unavailable",
            Self::Informational => "informational",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Leaf {
    path: String,
    shortcut: String,
    status: Status,
    gate: String,
    line: usize,
}

struct Reader<'a> {
    source: &'a str,
    pos: usize,
    end: usize,
    line_starts: &'a [usize],
}

impl<'a> Reader<'a> {
    fn new(source: &'a str, pos: usize, end: usize, line_starts: &'a [usize]) -> Self {
        Self {
            source,
            pos,
            end,
            line_starts,
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.end
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..self.end].chars().next()
    }

    fn rest(&self) -> &'a str {
        &self.source[self.pos..self.end]
    }

    fn line_at(&self, position: usize) -> usize {
        self.line_starts.partition_point(|start| *start <= position)
    }

    fn skip_trivia(&mut self) {
        loop {
            while let Some(character) = self.peek() {
                if character.is_whitespace() {
                    self.pos += character.len_utf8();
                } else {
                    break;
                }
            }
            if self.rest().starts_with("//") {
                self.pos = self
                    .rest()
                    .find('\n')
                    .map_or(self.end, |offset| self.pos + offset);
            } else if self.rest().starts_with("/*") {
                self.pos = self
                    .rest()
                    .find("*/")
                    .map_or(self.end, |offset| self.pos + offset + 2);
            } else {
                break;
            }
        }
    }

    fn skip_string(&mut self) {
        // The caller is positioned on the opening quote.
        self.pos += 1;
        while let Some(character) = self.peek() {
            if character == '\\' {
                self.pos += 1;
                if let Some(escaped) = self.peek() {
                    self.pos += escaped.len_utf8();
                }
            } else {
                self.pos += character.len_utf8();
                if character == '"' {
                    return;
                }
            }
        }
    }

    /// Finds the partner of the bracket at `open_index`, skipping strings and
    /// comments. `open_index` must point at `open`.
    fn find_matching(&self, open_index: usize, open: char, close: char) -> Option<usize> {
        let mut depth = 0usize;
        let mut index = open_index;
        while index < self.end {
            let character = self.source[index..self.end].chars().next()?;
            if character == '"' {
                let mut inner = Reader::new(self.source, index + 1, self.end, self.line_starts);
                inner.pos = index;
                inner.skip_string();
                index = inner.pos;
                continue;
            }
            if self.source[index..self.end].starts_with("//") {
                index = self.source[index..self.end]
                    .find('\n')
                    .map_or(self.end, |offset| index + offset);
                continue;
            }
            if self.source[index..self.end].starts_with("/*") {
                index = self.source[index..self.end]
                    .find("*/")
                    .map_or(self.end, |offset| index + offset + 2);
                continue;
            }
            if character == open {
                depth += 1;
            } else if character == close {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            index += character.len_utf8();
        }
        None
    }

    fn find_next_outside_strings(&self, from: usize, needle: char) -> Option<usize> {
        let mut index = from;
        while index < self.end {
            let character = self.source[index..self.end].chars().next()?;
            if character == '"' {
                let mut inner = Reader::new(self.source, index, self.end, self.line_starts);
                inner.skip_string();
                index = inner.pos;
                continue;
            }
            if character == needle {
                return Some(index);
            }
            index += character.len_utf8();
        }
        None
    }
}

/// Splits a comma-separated argument list without breaking nested groups or
/// string literals.
fn split_args(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    let mut index = 0usize;
    let bytes = text.as_bytes();
    while index < bytes.len() {
        let character = text[index..].chars().next().unwrap();
        if character == '"' {
            index += 1;
            while index < bytes.len() {
                let inner = text[index..].chars().next().unwrap();
                if inner == '\\' {
                    index += 2;
                } else if inner == '"' {
                    index += 1;
                    break;
                } else {
                    index += inner.len_utf8();
                }
            }
            continue;
        }
        match character {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
        index += character.len_utf8();
    }
    let tail = text[start..].trim();
    if !tail.is_empty() || !parts.is_empty() {
        parts.push(tail);
    }
    parts
}

fn string_literals(text: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut index = 0usize;
    while index < text.len() {
        let character = text[index..].chars().next().unwrap();
        if character == '"' {
            index += 1;
            let mut value = String::new();
            while index < text.len() {
                let inner = text[index..].chars().next().unwrap();
                if inner == '\\' {
                    index += 1;
                    if let Some(escaped) = text[index..].chars().next() {
                        value.push(escaped);
                        index += escaped.len_utf8();
                    }
                } else if inner == '"' {
                    index += 1;
                    break;
                } else {
                    value.push(inner);
                    index += inner.len_utf8();
                }
            }
            literals.push(value);
        } else {
            index += character.len_utf8();
        }
    }
    literals
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn format_label(expression: &str, env: &HashMap<String, String>) -> String {
    let trimmed = expression
        .trim()
        .trim_start_matches('&')
        .trim_start_matches("mut ");
    if trimmed.starts_with('"') {
        return string_literals(trimmed)
            .into_iter()
            .next()
            .unwrap_or_else(|| "<empty label>".to_owned());
    }
    if let Some(value) = env.get(trimmed) {
        return value.clone();
    }
    if trimmed.starts_with("format!") {
        let literals = string_literals(trimmed);
        if literals.is_empty() {
            return "<dynamic label>".to_owned();
        }
        return literals
            .join("<value>")
            .replace("{}", "<value>")
            .replace("{", "")
            .replace("}", "");
    }
    if let Some(name) = trimmed
        .trim_matches(|character: char| !character.is_alphanumeric() && character != '_')
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .next()
    {
        if !name.is_empty() {
            return format!("<{name}>");
        }
    }
    "<dynamic label>".to_owned()
}

fn status_from_gate(gate: &str) -> Status {
    match gate.trim() {
        "true" => Status::Available,
        "false" => Status::Unavailable,
        _ => Status::StateDependent,
    }
}

fn parse_for(
    reader: &mut Reader<'_>,
    path: &mut Vec<String>,
    env: &HashMap<String, String>,
    out: &mut Vec<Leaf>,
) -> bool {
    let start = reader.pos;
    let Some(in_offset) = reader.rest().find(" in ") else {
        return false;
    };
    let pattern = reader.rest()[3..in_offset].trim();
    let names: Vec<&str> = pattern
        .trim_matches(|character: char| character == '(' || character == ')' || character == '&')
        .split(',')
        .map(|name| name.trim().trim_start_matches("mut "))
        .filter(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|character| character.is_alphanumeric() || character == '_')
        })
        .collect();
    let mut cursor = start + in_offset + 4;
    let mut skip = Reader::new(reader.source, cursor, reader.end, reader.line_starts);
    skip.skip_trivia();
    if skip.peek() != Some('[') {
        return false;
    }
    let Some(array_end) = skip.find_matching(skip.pos, '[', ']') else {
        return false;
    };
    let array_body = &skip.source[skip.pos + 1..array_end];
    let items = split_args(array_body);
    cursor = array_end + 1;
    let mut body_reader = Reader::new(reader.source, cursor, reader.end, reader.line_starts);
    body_reader.skip_trivia();
    let body_start = body_reader.pos;
    if body_reader.peek() != Some('{') {
        return false;
    }
    let Some(body_end) = body_reader.find_matching(body_start, '{', '}') else {
        return false;
    };

    let mut environments = Vec::new();
    for item in items {
        let values = string_literals(item);
        let mut mapping = HashMap::new();
        for (index, name) in names.iter().enumerate() {
            if let Some(value) = values.get(index) {
                mapping.insert((*name).to_owned(), value.clone());
            }
        }
        if !mapping.is_empty() {
            environments.push(mapping);
        }
    }
    if environments.is_empty() {
        return false;
    }
    for mapping in environments {
        let mut merged = env.clone();
        merged.extend(mapping);
        let mut body = Reader::new(reader.source, body_start + 1, body_end, reader.line_starts);
        let mut local_path = path.clone();
        scan_scope(&mut body, &mut local_path, &merged, out);
    }
    reader.pos = body_end + 1;
    true
}

fn parse_menu_button(
    reader: &mut Reader<'_>,
    path: &mut Vec<String>,
    env: &HashMap<String, String>,
    out: &mut Vec<Leaf>,
) {
    let call_start = reader.pos;
    let Some(open) = reader.find_next_outside_strings(reader.pos, '(') else {
        reader.pos += 1;
        return;
    };
    let Some(close) = reader.find_matching(open, '(', ')') else {
        reader.pos = open + 1;
        return;
    };
    let arguments = &reader.source[open + 1..close];
    let args = split_args(arguments);
    let label = args
        .first()
        .map(|argument| format_label(argument, env))
        .unwrap_or_else(|| "<menu>".to_owned());
    let Some(body_start) = reader.find_next_outside_strings(open, '{') else {
        reader.pos = close + 1;
        return;
    };
    if body_start >= close {
        reader.pos = close + 1;
        return;
    }
    let Some(body_end) = reader.find_matching(body_start, '{', '}') else {
        reader.pos = close + 1;
        return;
    };
    let _ = call_start;
    path.push(label);
    let mut body = Reader::new(reader.source, body_start + 1, body_end, reader.line_starts);
    scan_scope(&mut body, path, env, out);
    path.pop();
    reader.pos = close + 1;
}

fn parse_leaf(
    reader: &mut Reader<'_>,
    path: &[String],
    env: &HashMap<String, String>,
    out: &mut Vec<Leaf>,
) {
    let call_start = reader.pos;
    let line = reader.line_at(call_start);
    let rest = reader.rest();
    let (name, is_leaf) = if rest.starts_with("checked_menu_command(") {
        ("checked_menu_command", true)
    } else if rest.starts_with("menu_command(") {
        ("menu_command", true)
    } else if rest.starts_with("disabled_menu_commands(") {
        ("disabled_menu_commands", true)
    } else if rest.starts_with("ui.add_enabled(") {
        ("ui.add_enabled", true)
    } else if rest.starts_with("ui.selectable_label(") {
        ("ui.selectable_label", true)
    } else if rest.starts_with("ui.button(") {
        ("ui.button", true)
    } else if rest.starts_with("ui.label(") {
        ("ui.label", true)
    } else {
        ("", false)
    };
    if !is_leaf {
        reader.pos += 1;
        return;
    }
    let Some(open) = reader.find_next_outside_strings(reader.pos, '(') else {
        reader.pos += 1;
        return;
    };
    let Some(close) = reader.find_matching(open, '(', ')') else {
        reader.pos = open + 1;
        return;
    };
    let args = split_args(&reader.source[open + 1..close]);
    let mut push = |label: String, shortcut: String, status: Status, gate: String| {
        out.push(Leaf {
            path: full_path(path, &label),
            shortcut,
            status,
            gate: collapse_whitespace(&gate),
            line,
        });
    };
    match name {
        "menu_command" => {
            let label = args
                .get(1)
                .map(|value| format_label(value, env))
                .unwrap_or_default();
            let shortcut = args
                .get(2)
                .map(|value| format_label(value, env))
                .filter(|value| !value.starts_with('<'))
                .unwrap_or_default();
            let gate = args.get(3).copied().unwrap_or("false");
            push(label, shortcut, status_from_gate(gate), gate.to_owned());
        }
        "checked_menu_command" => {
            let label = args
                .get(1)
                .map(|value| format_label(value, env))
                .unwrap_or_default();
            let shortcut = args
                .get(2)
                .map(|value| format_label(value, env))
                .filter(|value| !value.starts_with('<'))
                .unwrap_or_default();
            let gate = args.get(4).copied().unwrap_or("false");
            push(label, shortcut, status_from_gate(gate), gate.to_owned());
        }
        "disabled_menu_commands" => {
            let Some(array) = args.get(1) else {
                reader.pos = close + 1;
                return;
            };
            let stripped = array
                .trim()
                .trim_start_matches('&')
                .trim_matches(|character| character == '[' || character == ']');
            for entry in split_args(stripped) {
                if entry.is_empty() {
                    continue;
                }
                let values = string_literals(entry);
                let label = values
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "<row>".to_owned());
                let shortcut = values.get(1).cloned().unwrap_or_default();
                push(label, shortcut, Status::Unavailable, "false".to_owned());
            }
        }
        "ui.add_enabled" => {
            let gate = args.first().copied().unwrap_or("false");
            let label = args
                .get(1)
                .and_then(|button| string_literals(button).into_iter().next())
                .unwrap_or_else(|| "<button>".to_owned());
            push(
                label,
                String::new(),
                status_from_gate(gate),
                gate.to_owned(),
            );
        }
        "ui.selectable_label" => {
            let label = args
                .get(1)
                .map(|value| format_label(value, env))
                .unwrap_or_default();
            let gate = args.first().copied().unwrap_or("true");
            push(label, String::new(), Status::Available, gate.to_owned());
        }
        "ui.button" => {
            let label = args
                .first()
                .map(|value| format_label(value, env))
                .unwrap_or_default();
            push(label, String::new(), Status::Available, "true".to_owned());
        }
        "ui.label" => {
            let label = string_literals(args.first().copied().unwrap_or(""))
                .into_iter()
                .next()
                .unwrap_or_else(|| {
                    args.first()
                        .map(|value| format_label(value, env))
                        .unwrap_or_default()
                });
            push(label, String::new(), Status::Informational, String::new());
        }
        _ => {}
    }
    reader.pos = close + 1;
}

fn full_path(path: &[String], label: &str) -> String {
    if path.is_empty() {
        label.to_owned()
    } else {
        format!("{} > {label}", path.join(" > "))
    }
}

fn scan_scope(
    reader: &mut Reader<'_>,
    path: &mut Vec<String>,
    env: &HashMap<String, String>,
    out: &mut Vec<Leaf>,
) {
    loop {
        reader.skip_trivia();
        if reader.eof() {
            return;
        }
        if reader.peek() == Some('}') {
            reader.pos += 1;
            return;
        }
        if reader.peek() == Some('{') {
            // A nested block (`if`, `match` arm or closure we do not model as a
            // menu): scan its contents under the current path, then continue.
            let Some(close) = reader.find_matching(reader.pos, '{', '}') else {
                reader.pos += 1;
                continue;
            };
            let open = reader.pos;
            let mut inner = Reader::new(reader.source, open + 1, close, reader.line_starts);
            scan_scope(&mut inner, path, env, out);
            reader.pos = close + 1;
            continue;
        }
        if reader.rest().starts_with("for ") || reader.rest().starts_with("for(") {
            if parse_for(reader, path, env, out) {
                continue;
            }
        }
        if reader.rest().starts_with("ui.menu_button(") {
            parse_menu_button(reader, path, env, out);
            continue;
        }
        let leaf_start = [
            "checked_menu_command(",
            "menu_command(",
            "disabled_menu_commands(",
            "ui.add_enabled(",
            "ui.selectable_label(",
            "ui.button(",
            "ui.label(",
        ]
        .iter()
        .any(|needle| reader.rest().starts_with(needle));
        if leaf_start {
            parse_leaf(reader, path, env, out);
            continue;
        }
        match reader.peek() {
            Some('"') => reader.skip_string(),
            Some(character) => reader.pos += character.len_utf8(),
            None => return,
        }
    }
}

fn function_body<'a>(
    source: &'a str,
    signature: &str,
    line_starts: &'a [usize],
) -> Option<Reader<'a>> {
    let start = source.find(signature)?;
    let reader = Reader::new(source, start, source.len(), line_starts);
    let open = reader.find_next_outside_strings(start, '{')?;
    let close = reader.find_matching(open, '{', '}')?;
    Some(Reader::new(source, open + 1, close, line_starts))
}

fn line_offsets(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn parse_main_menu(source: &str) -> (Vec<Leaf>, Vec<Leaf>) {
    let line_starts = line_offsets(source);
    let mut leaves = Vec::new();
    if let Some(mut reader) = function_body(source, "fn show_menu(", &line_starts) {
        let mut path = Vec::new();
        scan_scope(&mut reader, &mut path, &HashMap::new(), &mut leaves);
    }
    let mut shell = Vec::new();
    if let Some(mut reader) = function_body(source, "fn show_empty_project_menu(", &line_starts) {
        let mut path = Vec::new();
        scan_scope(&mut reader, &mut path, &HashMap::new(), &mut shell);
    }
    (leaves, shell)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn count_status(leaves: &[Leaf], status: Status) -> usize {
    leaves.iter().filter(|leaf| leaf.status == status).count()
}

fn duplicate_paths(leaves: &[Leaf]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for leaf in leaves {
        *counts.entry(leaf.path.as_str()).or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(path, count)| (path.to_owned(), count))
        .collect()
}

fn render_menu_section(title: &str, leaves: &[Leaf], report: &mut String) {
    report.push_str(&format!("## {title}\n\n"));
    report.push_str(&format!(
        "{} leaves: {} available, {} state-dependent, {} unavailable, {} informational.\n\n",
        leaves.len(),
        count_status(leaves, Status::Available),
        count_status(leaves, Status::StateDependent),
        count_status(leaves, Status::Unavailable),
        count_status(leaves, Status::Informational),
    ));
    report.push_str("| Path | Shortcut | Status | Availability expression | Source line |\n");
    report.push_str("|---|---|---|---|---:|\n");
    let mut sorted: Vec<&Leaf> = leaves.iter().collect();
    sorted.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
    });
    for leaf in sorted {
        report.push_str(&format!(
            "| {} | {} | {} | `{}` | {} |\n",
            escape_table(&leaf.path),
            escape_table(&leaf.shortcut),
            leaf.status.label(),
            escape_table(&leaf.gate),
            leaf.line,
        ));
    }
    report.push('\n');
    let duplicates = duplicate_paths(leaves);
    if duplicates.is_empty() {
        report.push_str("No duplicate full paths.\n\n");
    } else {
        report.push_str("### Duplicate full paths\n\n");
        for (path, count) in duplicates {
            report.push_str(&format!("- `{path}` appears {count} times\n"));
        }
        report.push('\n');
    }
}

fn escape_table(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn format_chord(chord: &opentrack_core::commands::KeyChord) -> String {
    let mut parts = Vec::new();
    let modifiers = chord.modifiers();
    if modifiers.control {
        parts.push("Ctrl".to_owned());
    }
    if modifiers.alt {
        parts.push("Alt".to_owned());
    }
    if modifiers.shift {
        parts.push("Shift".to_owned());
    }
    if modifiers.windows {
        parts.push("Win".to_owned());
    }
    parts.push(match chord.key() {
        KeyCode::Letter(letter) => letter.to_string(),
        KeyCode::Digit(digit) => digit.to_string(),
        KeyCode::Function(number) => format!("F{number}"),
        KeyCode::Arrow(ArrowKey::Up) => "Up".to_owned(),
        KeyCode::Arrow(ArrowKey::Down) => "Down".to_owned(),
        KeyCode::Arrow(ArrowKey::Left) => "Left".to_owned(),
        KeyCode::Arrow(ArrowKey::Right) => "Right".to_owned(),
        KeyCode::Enter => "Enter".to_owned(),
        KeyCode::Escape => "Escape".to_owned(),
        KeyCode::Space => "Space".to_owned(),
        KeyCode::Insert => "Insert".to_owned(),
        KeyCode::Delete => "Delete".to_owned(),
        KeyCode::Backspace => "Backspace".to_owned(),
        KeyCode::Home => "Home".to_owned(),
        KeyCode::End => "End".to_owned(),
        KeyCode::Comma => ",".to_owned(),
        KeyCode::Minus => "-".to_owned(),
        KeyCode::Period => ".".to_owned(),
        KeyCode::Plus => "+".to_owned(),
        KeyCode::Slash => "/".to_owned(),
        KeyCode::NumpadDigit(digit) => format!("Num {digit}"),
        KeyCode::NumpadAdd => "Num +".to_owned(),
        KeyCode::NumpadSubtract => "Num -".to_owned(),
        KeyCode::NumpadMultiply => "Num *".to_owned(),
        KeyCode::NumpadDivide => "Num /".to_owned(),
        KeyCode::NumpadDecimal => "Num .".to_owned(),
        KeyCode::NumpadComma => "Num ,".to_owned(),
    });
    parts.join("+")
}

struct ShortcutRow {
    id: String,
    category: String,
    label: String,
    chords: Vec<String>,
    dispatched: bool,
}

fn shortcut_rows(
    catalog: &CommandCatalog,
    profile: &KeymapProfile,
    dispatch_source: &str,
) -> Vec<ShortcutRow> {
    let mut bindings: BTreeMap<String, (String, String, BTreeSet<String>)> = BTreeMap::new();
    for binding in profile.keymap.bindings() {
        let id = binding.command.as_str().to_owned();
        let definition = catalog.get(&binding.command);
        let category = definition
            .map(|item| item.category.clone())
            .unwrap_or_default();
        let label = definition
            .map(|item| item.label.clone())
            .unwrap_or_default();
        let entry = bindings
            .entry(id)
            .or_insert_with(|| (category, label, BTreeSet::new()));
        entry.2.insert(format_chord(&binding.chord));
    }
    bindings
        .into_iter()
        .map(|(id, (category, label, chords))| ShortcutRow {
            dispatched: dispatch_source.contains(&format!("\"{id}\" =>")),
            id,
            category,
            label,
            chords: chords.into_iter().collect(),
        })
        .collect()
}

fn render_shortcut_section(rows: &[ShortcutRow], report: &mut String) {
    let dispatched = rows.iter().filter(|row| row.dispatched).count();
    report.push_str("## Bundled Windows reference keymap\n\n");
    report.push_str(&format!(
        "{} distinct command IDs; {dispatched} have a `dispatch_reference_command` arm; {} have none.\n\n",
        rows.len(),
        rows.len() - dispatched,
    ));
    report.push_str(
        "A dispatch arm only proves the command reaches code in that function. It does not prove the\n\
         resulting action is complete, and it does not cover commands intercepted before dispatch.\n\n",
    );
    report.push_str(
        "| Command ID | Category | Label | Chords | Dispatch arm |\n|---|---|---|---|---|\n",
    );
    for row in rows {
        report.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            row.id,
            escape_table(&row.category),
            escape_table(&row.label),
            escape_table(&row.chords.join(", ")),
            if row.dispatched { "yes" } else { "no" },
        ));
    }
    report.push('\n');
    let missing: Vec<&ShortcutRow> = rows.iter().filter(|row| !row.dispatched).collect();
    if missing.is_empty() {
        report.push_str("No bound command lacks a dispatch arm.\n\n");
    } else {
        report.push_str("### Bound commands without a dispatch arm\n\n");
        for row in missing {
            report.push_str(&format!(
                "- {} (`{}`, {}) — `{}`\n",
                row.label,
                row.chords.join(", "),
                row.category,
                row.id,
            ));
        }
        report.push('\n');
    }
}

fn build_report() -> (String, Vec<Leaf>, Vec<Leaf>, Vec<ShortcutRow>) {
    let source = include_str!("app.rs");
    let (leaves, shell) = parse_main_menu(source);
    let profile = KeymapProfile::windows_reference_default().expect("bundled keymap");
    let shortcuts = shortcut_rows(&profile.catalog, &profile, source);
    let keymap = include_str!("../../../docs/reference-keymap-windows.csv");

    let mut report = String::new();
    report.push_str("# Generated main-menu and shortcut inventory\n\n");
    report.push_str(
        "Generated from `crates/opentrack-app/src/app.rs` and the bundled Windows reference keymap.\n\
         Do not edit by hand. Regenerate after changing either source:\n\n\
         ```text\n\
         $env:UPDATE_SOURCE_INVENTORY = \"1\"\n\
         cargo test -p opentrack generated_source_inventory -- --nocapture\n\
         ```\n\n",
    );
    report.push_str(&format!(
        "- Source snapshot: `app.rs` FNV-1a 64 `0x{:016x}`; keymap CSV `0x{:016x}`\n",
        fnv1a64(source.as_bytes()),
        fnv1a64(keymap.as_bytes()),
    ));
    report.push_str(
        "- Scope: the project-open main menu and the empty-project shell. Context menus are\n\
         enumerated in the curated `docs/reviews/audit-menus.md` register; this generated file\n\
         does not claim to cover them.\n\
         - `available` means the row is built with literal `true`; it is still subject to the\n\
         menu being open. `state-dependent` means the availability is project/selection state.\n\
         `unavailable` means literal `false` or `disabled_menu_commands`. `informational` rows\n\
         are captions, not commands.\n\
         - Generated families are expanded from the simple `for` arrays in the builder. A\n\
         `<name>` path is a runtime-generated label (workspace/template/device family) and is\n\
         counted once, not per runtime instance.\n\n",
    );
    render_menu_section("Project-open main menu", &leaves, &mut report);
    render_menu_section("Empty-project shell", &shell, &mut report);
    render_shortcut_section(&shortcuts, &mut report);
    (report, leaves, shell, shortcuts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn generated_source_inventory_matches_source() {
        let (report, leaves, shell, shortcuts) = build_report();
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REPORT_PATH);
        if std::env::var_os("UPDATE_SOURCE_INVENTORY").is_some() {
            std::fs::write(&path, &report).expect("write generated inventory");
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "cannot read {}: {error}. Run with UPDATE_SOURCE_INVENTORY=1.",
                path.display()
            )
        });
        assert_eq!(
            committed.replace("\r\n", "\n"),
            report.replace("\r\n", "\n"),
            "regenerate docs/reviews/generated-source-inventory.md"
        );
        eprintln!(
            "menu inventory: {} project-open leaves, {} empty-shell leaves, {} bound command IDs",
            leaves.len(),
            shell.len(),
            shortcuts.len()
        );
    }

    #[test]
    fn generated_inventory_is_internally_consistent() {
        let (_, leaves, shell, shortcuts) = build_report();
        assert!(leaves.len() > 150, "main menu unexpectedly small");
        assert!(shell.len() > 15, "empty shell unexpectedly small");
        assert_eq!(shortcuts.len(), 152, "bundled keymap command count changed");

        assert_eq!(
            leaves
                .iter()
                .filter(|leaf| leaf.status == Status::Unavailable)
                .count(),
            leaves
                .iter()
                .filter(|leaf| leaf.status == Status::Unavailable)
                .map(|leaf| leaf.path.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            "an unavailable leaf shares a full path"
        );
        assert!(duplicate_paths(&leaves).is_empty(), "duplicate menu paths");
        assert!(duplicate_paths(&shell).is_empty(), "duplicate shell paths");

        for leaf in leaves.iter().chain(shell.iter()) {
            assert!(!leaf.path.is_empty());
            assert!(leaf.line > 0);
            if matches!(leaf.status, Status::Available | Status::StateDependent) {
                assert!(!leaf.gate.is_empty(), "{} has no availability", leaf.path);
            }
        }
        for menu in [
            "File",
            "Edit",
            "Project",
            "Audio",
            "MIDI",
            "Scores",
            "Media",
            "Transport",
            "Devices",
            "Window",
            "Help",
        ] {
            assert!(
                leaves
                    .iter()
                    .any(|leaf| leaf.path.starts_with(&format!("{menu} > "))),
                "missing {menu} menu"
            );
        }
        for menu in ["MIDI", "Scores", "Media", "Transport", "Window"] {
            assert!(
                shell
                    .iter()
                    .any(|leaf| leaf.path == format!("{menu} > No project is open")),
                "missing empty-shell row for {menu}"
            );
        }
        let missing = shortcuts.iter().filter(|row| !row.dispatched).count();
        assert_eq!(missing, 5, "bound command dispatch gap changed");
        for (id, path) in [
            ("devices.video", "Devices > Video Window"),
            ("edit.open_score_editor", "MIDI > Open Score Editor"),
            ("project.open_tempo_track_editor", "Project > Tempo Track"),
            (
                "transport.retrospective_record",
                "Transport > Retrospective Record",
            ),
            (
                "transport.use_external_sync",
                "Transport > Use External Sync",
            ),
        ] {
            assert!(
                shortcuts.iter().any(|row| row.id == id && !row.dispatched),
                "{id} is no longer an uncovered bound command"
            );
            let leaf = leaves
                .iter()
                .find(|leaf| leaf.path == path)
                .unwrap_or_else(|| panic!("{path} missing from the menu inventory"));
            assert_eq!(
                leaf.status,
                Status::Unavailable,
                "{path} is not a literal unavailable row"
            );
        }
    }
}
