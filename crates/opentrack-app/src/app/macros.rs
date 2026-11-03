//! User command macros: a named, ordered list of fixed command IDs run through
//! the ordinary keymap dispatch path.
//!
//! Macros store command IDs only, so they cannot drift into a second scripting
//! language or capture stale parameter values: each step runs the same code a
//! shortcut runs. Marker-slot commands need the digit from the physical chord
//! and are therefore not offered as steps.

use super::*;
use serde::{Deserialize, Serialize};

/// Most macros a project keeps.
pub(super) const MAX_MACROS: usize = 32;
/// Most steps one macro may hold.
pub(super) const MAX_MACRO_STEPS: usize = 32;
/// Longest macro name.
pub(super) const MAX_MACRO_NAME: usize = 64;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub(super) struct Macro {
    pub name: String,
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Trims names, drops unnamed macros, caps steps and de-duplicates names.
pub(super) fn sanitize_macros(mut macros: Vec<Macro>) -> Vec<Macro> {
    macros.retain(|entry| !entry.name.trim().is_empty());
    for entry in &mut macros {
        entry.name = entry
            .name
            .trim()
            .chars()
            .take(MAX_MACRO_NAME)
            .collect::<String>();
        entry.commands.truncate(MAX_MACRO_STEPS);
    }
    let mut seen = std::collections::BTreeSet::new();
    macros.retain(|entry| seen.insert(entry.name.to_lowercase()));
    macros.truncate(MAX_MACROS);
    macros
}

/// Commands that can run without the physical chord that triggered them.
pub(super) fn replayable_command(id: &str) -> bool {
    !id.starts_with("transport.set_marker_")
        && !id.starts_with("transport.to_marker_")
        && !id.starts_with("transport.recall_cycle_marker_")
}

impl OpenTrackApp {
    /// Command IDs and labels available as macro steps, filtered and sorted.
    fn macro_picker_entries(&self) -> Vec<(String, String)> {
        let filter = self.macro_filter.trim().to_lowercase();
        let Some(profile) = self.keymap_profile.as_ref() else {
            return Vec::new();
        };
        let mut entries = profile
            .catalog
            .iter()
            .filter(|definition| replayable_command(definition.id.as_str()))
            .map(|definition| (definition.id.as_str().to_owned(), definition.label.clone()))
            .filter(|(id, label)| {
                filter.is_empty()
                    || id.to_lowercase().contains(&filter)
                    || label.to_lowercase().contains(&filter)
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.1
                .to_lowercase()
                .cmp(&right.1.to_lowercase())
                .then_with(|| left.0.cmp(&right.0))
        });
        entries
    }

    fn macro_label(&self, id: &str) -> String {
        self.keymap_profile
            .as_ref()
            .and_then(|profile| profile.catalog.get(&CommandId::new(id)))
            .map(|definition| definition.label.clone())
            .unwrap_or_else(|| id.to_owned())
    }

    /// Creates a macro from the pending name; refuses duplicates and blanks.
    fn create_macro(&mut self) {
        let name = self.macro_new_name.trim().to_owned();
        if name.is_empty() {
            self.status_message = "Name the macro before creating it".to_owned();
            return;
        }
        if self.macros.len() >= MAX_MACROS {
            self.status_message = format!("At most {MAX_MACROS} macros are supported");
            return;
        }
        if self
            .macros
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(&name))
        {
            self.status_message = format!("A macro named '{name}' already exists");
            return;
        }
        self.macros.push(Macro {
            name: name.clone(),
            commands: Vec::new(),
        });
        self.macro_selected = Some(self.macros.len() - 1);
        self.macro_new_name.clear();
        self.status_message = format!("Created macro '{name}'; add commands to it");
    }

    fn delete_selected_macro(&mut self) {
        let Some(index) = self.macro_selected else {
            return;
        };
        if index >= self.macros.len() {
            return;
        }
        let removed = self.macros.remove(index);
        self.macro_selected = if self.macros.is_empty() {
            None
        } else {
            Some(index.min(self.macros.len() - 1))
        };
        self.status_message = format!("Deleted macro '{}'", removed.name);
    }

    fn add_macro_step(&mut self, id: &str) {
        let Some(index) = self.macro_selected else {
            self.status_message = "Select a macro before adding a command".to_owned();
            return;
        };
        if !replayable_command(id) {
            self.status_message =
                "That command needs a physical shortcut (marker slots are not macro steps)"
                    .to_owned();
            return;
        }
        let label = self.macro_label(id);
        let Some(entry) = self.macros.get_mut(index) else {
            return;
        };
        if entry.commands.len() >= MAX_MACRO_STEPS {
            self.status_message = format!("A macro holds at most {MAX_MACRO_STEPS} commands");
            return;
        }
        entry.commands.push(id.to_owned());
        self.status_message = format!("Added '{label}' to '{}'", entry.name);
    }

    fn remove_macro_step(&mut self, step: usize) {
        let Some(index) = self.macro_selected else {
            return;
        };
        let Some(entry) = self.macros.get_mut(index) else {
            return;
        };
        if step < entry.commands.len() {
            entry.commands.remove(step);
        }
    }

    fn move_macro_step(&mut self, step: usize, delta: isize) {
        let Some(index) = self.macro_selected else {
            return;
        };
        let Some(entry) = self.macros.get_mut(index) else {
            return;
        };
        let target = step as isize + delta;
        if target < 0 || target as usize >= entry.commands.len() {
            return;
        }
        entry.commands.swap(step, target as usize);
    }

    /// Runs one macro through the ordinary dispatch path.
    fn run_macro(&mut self, index: usize, ctx: &egui::Context) {
        let Some(entry) = self.macros.get(index).cloned() else {
            return;
        };
        if !self.project_open {
            self.status_message = "Open a project before running a macro".to_owned();
            return;
        }
        if entry.commands.is_empty() {
            self.status_message = format!("Macro '{}' has no commands", entry.name);
            return;
        }
        // Marker-slot entries cannot have been added, but guard against loaded
        // files anyway: a step that needs a chord would silently do nothing.
        let chord = KeyChord::new(KeyCode::Escape, KeyModifiers::default());
        let mut ran = 0usize;
        for id in &entry.commands {
            if !replayable_command(id) {
                continue;
            }
            let label = self.macro_label(id);
            self.dispatch_reference_command(id, &label, chord, ctx);
            ran += 1;
        }
        self.status_message = format!("Ran macro '{}' ({ran} command(s))", entry.name);
    }

    pub(super) fn show_macros(&mut self, ctx: &egui::Context) {
        if !self.macros_open {
            return;
        }
        let macros = self.macros.clone();
        let mut open = true;
        let mut selected = self.macro_selected;
        let mut create = false;
        let mut delete = false;
        let mut run = None;
        let mut add_step = None;
        let mut remove_step = None;
        let mut move_step = None;
        let mut new_name = self.macro_new_name.clone();
        let mut filter = self.macro_filter.clone();
        let entries = self.macro_picker_entries();
        let labels = macros
            .iter()
            .map(|entry| {
                entry
                    .commands
                    .iter()
                    .map(|id| self.macro_label(id))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let selected_name = selected
            .filter(|index| *index < macros.len())
            .map(|index| macros[index].name.clone())
            .unwrap_or_else(|| "No macro selected".to_owned());
        let response = egui::Window::new("Macros")
            .id(Id::new("command-macros"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(560.0, 460.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_min_width(520.0);
                ui.label(
                    egui::RichText::new(
                        "A macro is a named list of fixed commands run in order through the normal shortcuts. Macros are saved with the application settings.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Macro");
                    egui::ComboBox::from_id_salt("macro-select")
                        .width(240.0)
                        .selected_text(selected_name.clone())
                        .show_ui(ui, |ui| {
                            for (index, entry) in macros.iter().enumerate() {
                                let label =
                                    format!("{} ({} step(s))", entry.name, entry.commands.len());
                                if ui
                                    .selectable_label(selected == Some(index), label)
                                    .clicked()
                                {
                                    selected = Some(index);
                                }
                            }
                        });
                    if ui
                        .add_enabled(selected.is_some(), egui::Button::new("Delete Macro"))
                        .clicked()
                    {
                        delete = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut new_name)
                            .hint_text("New macro name")
                            .desired_width(200.0),
                    );
                    if ui.button("Create").clicked() {
                        create = true;
                    }
                });
                ui.separator();
                let Some(index) = selected.filter(|index| *index < macros.len()) else {
                    ui.label("Select or create a macro.");
                    return;
                };
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !macros[index].commands.is_empty(),
                            egui::Button::new("Run"),
                        )
                        .clicked()
                    {
                        run = Some(index);
                    }
                    ui.label(format!(
                        "{} step(s) in '{}'",
                        macros[index].commands.len(),
                        macros[index].name
                    ));
                });
                ui.label(egui::RichText::new("Steps").strong());
                for (step, label) in labels[index].iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}. {label}", step + 1));
                        if ui.small_button("\u{2191}").clicked() {
                            move_step = Some((step, -1));
                        }
                        if ui.small_button("\u{2193}").clicked() {
                            move_step = Some((step, 1));
                        }
                        if ui.small_button("\u{2715}").clicked() {
                            remove_step = Some(step);
                        }
                    });
                }
                ui.separator();
                ui.label(egui::RichText::new("Add a command").strong());
                ui.add(
                    egui::TextEdit::singleline(&mut filter)
                        .hint_text("Search commands")
                        .desired_width(240.0),
                );
                egui::ScrollArea::vertical()
                    .id_salt("macro-command-list")
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for (id, label) in &entries {
                            if ui
                                .selectable_label(false, label)
                                .on_hover_text(id)
                                .clicked()
                            {
                                add_step = Some(id.clone());
                            }
                        }
                    });
            });
        dismiss_window_on_outside_click(
            ctx,
            Id::new("command-macros"),
            &mut open,
            response.as_ref(),
        );
        self.macro_new_name = new_name;
        self.macro_filter = filter;
        self.macro_selected = selected;
        if create {
            self.create_macro();
        }
        if delete {
            self.delete_selected_macro();
        }
        if let Some(id) = add_step {
            self.add_macro_step(&id);
        }
        if let Some(step) = remove_step {
            self.remove_macro_step(step);
        }
        if let Some((step, delta)) = move_step {
            self.move_macro_step(step, delta);
        }
        if let Some(index) = run {
            self.run_macro(index, ctx);
        }
        self.macros_open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app
    }

    #[test]
    fn macros_are_sanitized_and_bounded() {
        let mut macros = vec![
            Macro {
                name: "  Trim Me  ".to_owned(),
                commands: vec!["a".to_owned(); MAX_MACRO_STEPS + 5],
            },
            Macro {
                name: "   ".to_owned(),
                commands: Vec::new(),
            },
            Macro {
                name: "trim me".to_owned(),
                commands: vec!["b".to_owned()],
            },
        ];
        macros.extend((0..40).map(|index| Macro {
            name: format!("extra {index}"),
            commands: Vec::new(),
        }));
        let sanitized = sanitize_macros(macros);
        assert_eq!(sanitized.len(), MAX_MACROS);
        assert_eq!(sanitized[0].name, "Trim Me");
        assert_eq!(sanitized[0].commands.len(), MAX_MACRO_STEPS);
        assert!(
            !sanitized
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case("trim me") && entry.name != "Trim Me"),
            "duplicate names are dropped"
        );

        assert!(!replayable_command("transport.set_marker_1"));
        assert!(!replayable_command("transport.to_marker_3_to_9"));
        assert!(!replayable_command("transport.recall_cycle_marker_1_to_9"));
        assert!(replayable_command("transport.cycle"));
    }

    #[test]
    fn a_macro_creates_adds_steps_and_runs_them_in_order() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.macro_new_name = "Toggle helpers".to_owned();
        app.create_macro();
        assert_eq!(app.macros.len(), 1);
        assert_eq!(app.macros[0].name, "Toggle helpers");
        assert_eq!(app.macro_selected, Some(0));

        // Duplicates and blanks are refused.
        app.macro_new_name = "toggle helpers".to_owned();
        app.create_macro();
        assert_eq!(app.macros.len(), 1);
        assert!(app.status_message.contains("already exists"));
        app.macro_new_name = "  ".to_owned();
        app.create_macro();
        assert_eq!(app.macros.len(), 1);

        app.add_macro_step("edit.auto_select_events_under_cursor");
        app.add_macro_step("transport.cycle");
        assert_eq!(app.macros[0].commands.len(), 2);
        app.add_macro_step("transport.set_marker_1");
        assert_eq!(app.macros[0].commands.len(), 2, "marker steps are refused");

        // Steps keep their order and can be reordered or removed.
        app.move_macro_step(1, -1);
        assert_eq!(app.macros[0].commands[0], "transport.cycle");
        app.move_macro_step(0, 1);
        assert_eq!(
            app.macros[0].commands[0],
            "edit.auto_select_events_under_cursor"
        );
        app.remove_macro_step(1);
        assert_eq!(app.macros[0].commands.len(), 1);

        // Running uses the ordinary dispatch path: the autoselect flag flips.
        let before = app.auto_select_events_under_cursor;
        app.run_macro(0, &ctx);
        assert_ne!(app.auto_select_events_under_cursor, before);
        assert!(
            app.status_message
                .contains("Ran macro 'Toggle helpers' (1 command(s))")
        );

        // An empty macro reports instead of pretending to run.
        app.macros[0].commands.clear();
        app.run_macro(0, &ctx);
        assert!(app.status_message.contains("has no commands"));
    }

    #[test]
    fn the_window_lists_macros_and_steps() {
        let ctx = egui::Context::default();
        let mut app = app();
        app.macros = vec![Macro {
            name: "Session start".to_owned(),
            commands: vec!["transport.cycle".to_owned()],
        }];
        app.macro_selected = Some(0);
        app.macros_open = true;
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 760.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_macros(&ctx),
            );
            output.textures_delta.clear();
            if frame == 0 {
                continue;
            }
            fn walk(shape: &egui::Shape, texts: &mut Vec<String>) {
                match shape {
                    egui::Shape::Text(text) => texts.push(text.galley.text().to_owned()),
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            walk(shape, texts);
                        }
                    }
                    _ => {}
                }
            }
            for shape in &output.shapes {
                walk(&shape.shape, &mut texts);
            }
        }
        let joined = texts.join("\n");
        assert!(joined.contains("Macros"), "{joined}");
        assert!(joined.contains("Session start"), "{joined}");
        assert!(joined.contains("1. Cycle"), "{joined}");
    }
}
