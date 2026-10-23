//! Command definitions and context-sensitive keyboard bindings.
//!
//! This module is independent of any GUI event library. The application shell
//! translates native key events into these stable domain types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Stable identifier for a built-in command or user macro.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CommandId(String);

impl CommandId {
    /// Creates an identifier such as `edit.copy` or `transport.start_stop`.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the stable textual identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The implementation associated with a command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandAction {
    /// A built-in operation implemented by the application.
    BuiltIn,
    /// A saved macro that dispatches commands in sequence.
    Macro(Vec<CommandId>),
}

/// User-visible metadata and action for a command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandDefinition {
    pub id: CommandId,
    pub label: String,
    pub category: String,
    pub action: CommandAction,
}

/// Registry of built-in commands and saved macros.
#[derive(Clone, Debug, Default)]
pub struct CommandCatalog {
    definitions: BTreeMap<CommandId, CommandDefinition>,
}

impl CommandCatalog {
    /// Adds or replaces a command definition, returning the previous value.
    pub fn register(&mut self, definition: CommandDefinition) -> Option<CommandDefinition> {
        self.definitions.insert(definition.id.clone(), definition)
    }

    /// Looks up a command by stable identifier.
    pub fn get(&self, id: &CommandId) -> Option<&CommandDefinition> {
        self.definitions.get(id)
    }

    /// Iterates over definitions in stable identifier order.
    pub fn iter(&self) -> impl Iterator<Item = &CommandDefinition> {
        self.definitions.values()
    }
}

/// A key independent of the selected GUI event library.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum KeyCode {
    Letter(char),
    Digit(u8),
    Function(u8),
    Arrow(ArrowKey),
    Enter,
    Escape,
    Space,
    Insert,
    Delete,
    Backspace,
    Home,
    End,
    Comma,
    Minus,
    Period,
    Plus,
    Slash,
    NumpadDigit(u8),
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
}

/// Directional keys.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ArrowKey {
    Up,
    Down,
    Left,
    Right,
}

/// Keyboard modifiers represented independently so they can be serialized.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct KeyModifiers {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub windows: bool,
}

/// One key and modifier combination.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct KeyChord {
    key: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyChord {
    pub fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        let key = match key {
            KeyCode::Letter(character) => KeyCode::Letter(character.to_ascii_uppercase()),
            other => other,
        };
        Self { key, modifiers }
    }

    pub fn key(self) -> KeyCode {
        self.key
    }

    pub fn modifiers(self) -> KeyModifiers {
        self.modifiers
    }
}

/// Where a key binding applies. Multiple contexts can be supplied when a
/// command is available in both a child editor and its parent window.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BindingScope {
    /// Applies in every application context.
    Global,
    /// Applies in any listed active context.
    Contexts(Vec<String>),
}

impl BindingScope {
    fn includes(&self, context: &str) -> bool {
        match self {
            Self::Global => false,
            Self::Contexts(contexts) => contexts.iter().any(|item| item == context),
        }
    }

    fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Global, _) | (_, Self::Global) => true,
            (Self::Contexts(left), Self::Contexts(right)) => left
                .iter()
                .any(|context| right.iter().any(|other| context == other)),
        }
    }
}

/// A shortcut assigned to a command in one or more application contexts.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyBinding {
    pub command: CommandId,
    pub chord: KeyChord,
    pub scope: BindingScope,
}

/// User-editable set of key bindings. Multiple chords may map to one command;
/// duplicate chords are retained so the assignment UI can report conflicts.
#[derive(Clone, Debug, Default)]
pub struct Keymap {
    bindings: Vec<KeyBinding>,
}

/// A command catalog paired with its built-in Windows reference bindings.
#[derive(Clone, Debug)]
pub struct KeymapProfile {
    pub catalog: CommandCatalog,
    pub keymap: Keymap,
}

impl KeymapProfile {
    /// Builds the built-in Windows profile from the checked-in manual extract.
    pub fn windows_reference_default() -> Result<Self, String> {
        let mut catalog = CommandCatalog::default();
        let mut keymap = Keymap::default();
        for (line_index, line) in REFERENCE_WINDOWS_KEYMAP.lines().enumerate().skip(1) {
            if line.trim().is_empty() {
                continue;
            }
            let fields: Vec<_> = line.splitn(4, ',').collect();
            if fields.len() != 4 {
                return Err(format!(
                    "reference keymap row {} must have four fields",
                    line_index + 1
                ));
            }
            let category = fields[1].trim();
            let label = fields[2].trim();
            let command = CommandId::new(format!(
                "{}.{}",
                identifier_part(category),
                identifier_part(label)
            ));
            catalog.register(CommandDefinition {
                id: command.clone(),
                label: label.to_owned(),
                category: category.to_owned(),
                action: CommandAction::BuiltIn,
            });

            let scope = if command.as_str() == "edit.open_close_editor" {
                BindingScope::Contexts(vec!["editor".to_owned()])
            } else {
                BindingScope::Global
            };
            for binding in fields[3].split(';').map(str::trim) {
                if binding.is_empty() {
                    return Err(format!(
                        "reference keymap row {} contains an empty binding",
                        line_index + 1
                    ));
                }
                for expanded in expand_binding_range(binding)
                    .map_err(|error| format!("reference keymap row {}: {error}", line_index + 1))?
                {
                    let chord = parse_key_chord(&expanded).map_err(|error| {
                        format!("reference keymap row {}: {error}", line_index + 1)
                    })?;
                    keymap.bind(KeyBinding {
                        command: command.clone(),
                        chord,
                        scope: scope.clone(),
                    });
                }
            }
        }

        Ok(Self { catalog, keymap })
    }
}

/// Result of resolving a chord in the current context stack.
#[derive(Clone, Debug, PartialEq)]
pub enum BindingResolution<'a> {
    Unbound,
    Bound(&'a KeyBinding),
    Conflict(Vec<&'a KeyBinding>),
}

const REFERENCE_WINDOWS_KEYMAP: &str = include_str!("../../../docs/reference-keymap-windows.csv");

fn identifier_part(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator && !output.is_empty() {
            output.push('_');
            last_was_separator = true;
        }
    }
    while output.ends_with('_') {
        output.pop();
    }
    output
}

fn expand_binding_range(binding: &str) -> Result<Vec<String>, String> {
    let Some((left, right)) = binding.split_once("..") else {
        return Ok(vec![binding.to_owned()]);
    };
    if right.len() != 1 || !right.as_bytes()[0].is_ascii_digit() {
        return Err(format!("unsupported shortcut range '{binding}'"));
    }
    let Some(start) = left.as_bytes().last().filter(|byte| byte.is_ascii_digit()) else {
        return Err(format!("unsupported shortcut range '{binding}'"));
    };
    let start = start - b'0';
    let end = right.as_bytes()[0] - b'0';
    if start > end {
        return Err(format!("descending shortcut range '{binding}'"));
    }
    let prefix = &left[..left.len() - 1];
    Ok((start..=end)
        .map(|digit| format!("{prefix}{digit}"))
        .collect())
}

fn parse_key_chord(binding: &str) -> Result<KeyChord, String> {
    let parts: Vec<_> = binding.split('+').collect();
    let Some(key_name) = parts.last() else {
        return Err(format!("empty shortcut '{binding}'"));
    };
    let mut modifiers = KeyModifiers::default();
    for modifier in &parts[..parts.len() - 1] {
        match *modifier {
            "Ctrl" => modifiers.control = true,
            "Alt" => modifiers.alt = true,
            "Shift" => modifiers.shift = true,
            "Win" => modifiers.windows = true,
            _ => return Err(format!("unknown modifier '{modifier}' in '{binding}'")),
        }
    }
    let key = parse_key_code(key_name)
        .ok_or_else(|| format!("unknown key '{key_name}' in '{binding}'"))?;
    Ok(KeyChord::new(key, modifiers))
}

fn parse_key_code(name: &str) -> Option<KeyCode> {
    let bytes = name.as_bytes();
    if bytes.len() == 1 {
        let byte = bytes[0];
        if byte.is_ascii_alphabetic() {
            return Some(KeyCode::Letter(char::from(byte)));
        }
        if byte.is_ascii_digit() {
            return Some(KeyCode::Digit(byte - b'0'));
        }
    }
    if let Some(number) = name.strip_prefix('F').and_then(|value| value.parse().ok()) {
        if (1..=35).contains(&number) {
            return Some(KeyCode::Function(number));
        }
    }
    if let Some(number) = name
        .strip_prefix("Numpad")
        .and_then(|value| value.parse::<u8>().ok())
    {
        if number <= 9 {
            return Some(KeyCode::NumpadDigit(number));
        }
    }
    match name {
        "Enter" | "Return" => Some(KeyCode::Enter),
        "Escape" | "Esc" => Some(KeyCode::Escape),
        "Space" => Some(KeyCode::Space),
        "Insert" => Some(KeyCode::Insert),
        "Delete" => Some(KeyCode::Delete),
        "Backspace" => Some(KeyCode::Backspace),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "Period" => Some(KeyCode::Period),
        "Comma" => Some(KeyCode::Comma),
        "Minus" => Some(KeyCode::Minus),
        "Plus" => Some(KeyCode::Plus),
        "Slash" => Some(KeyCode::Slash),
        "Left" => Some(KeyCode::Arrow(ArrowKey::Left)),
        "Right" => Some(KeyCode::Arrow(ArrowKey::Right)),
        "Up" => Some(KeyCode::Arrow(ArrowKey::Up)),
        "Down" => Some(KeyCode::Arrow(ArrowKey::Down)),
        "NumpadAdd" => Some(KeyCode::NumpadAdd),
        "NumpadSubtract" => Some(KeyCode::NumpadSubtract),
        "NumpadMultiply" => Some(KeyCode::NumpadMultiply),
        "NumpadSlash" => Some(KeyCode::NumpadDivide),
        "NumpadDecimal" => Some(KeyCode::NumpadDecimal),
        "NumpadComma" => Some(KeyCode::NumpadComma),
        _ => None,
    }
}

impl Keymap {
    /// Adds a binding without silently displacing an existing assignment.
    pub fn bind(&mut self, binding: KeyBinding) {
        self.bindings.push(binding);
    }

    /// Returns every binding in the map.
    pub fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    /// Replaces all bindings, normalizing letter case as `KeyChord::new` does.
    pub fn replace_bindings(&mut self, bindings: Vec<KeyBinding>) {
        self.bindings = bindings
            .into_iter()
            .map(|mut binding| {
                binding.chord = KeyChord::new(binding.chord.key, binding.chord.modifiers);
                binding
            })
            .collect();
    }

    /// Removes one binding by its full assignment and returns whether it existed.
    pub fn remove_binding(&mut self, binding: &KeyBinding) -> bool {
        let Some(index) = self.bindings.iter().position(|item| item == binding) else {
            return false;
        };
        self.bindings.remove(index);
        true
    }

    /// Removes and returns the binding at the current display-order index.
    pub fn remove_binding_at(&mut self, index: usize) -> Option<KeyBinding> {
        (index < self.bindings.len()).then(|| self.bindings.remove(index))
    }

    /// Finds bindings that would compete with a proposed assignment.
    pub fn conflicts_for(
        &self,
        command: &CommandId,
        chord: KeyChord,
        scope: &BindingScope,
    ) -> Vec<&KeyBinding> {
        self.bindings
            .iter()
            .filter(|binding| {
                binding.command != *command
                    && binding.chord == chord
                    && binding.scope.overlaps(scope)
            })
            .collect()
    }

    /// Resolves a chord using active contexts from most specific to broadest,
    /// then falls back to an application-wide binding. Ambiguous assignments
    /// are reported rather than silently selecting the first command.
    pub fn resolve(&self, chord: KeyChord, active_contexts: &[&str]) -> BindingResolution<'_> {
        for context in active_contexts {
            let matches: Vec<_> = self
                .bindings
                .iter()
                .filter(|binding| binding.chord == chord && binding.scope.includes(context))
                .collect();
            if !matches.is_empty() {
                return classify_matches(matches);
            }
        }

        let matches: Vec<_> = self
            .bindings
            .iter()
            .filter(|binding| binding.chord == chord && binding.scope == BindingScope::Global)
            .collect();
        classify_matches(matches)
    }
}

fn classify_matches<'a>(matches: Vec<&'a KeyBinding>) -> BindingResolution<'a> {
    let Some(first) = matches.first().copied() else {
        return BindingResolution::Unbound;
    };

    if matches
        .iter()
        .all(|binding| binding.command == first.command)
    {
        BindingResolution::Bound(first)
    } else {
        BindingResolution::Conflict(matches)
    }
}
