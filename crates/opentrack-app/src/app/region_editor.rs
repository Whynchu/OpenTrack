//! Editable project regions, presented separately from audio-event selection.
//!
//! Regions are navigation ranges, not clips. The editor keeps its draft and
//! selection transient; Apply validates the whole draft and commits one history
//! step. Cancelling or closing without Apply changes nothing.

use super::*;

#[derive(Clone)]
pub(super) struct RegionDraft {
    pub id: u64,
    pub name: String,
    pub start_beats: f64,
    pub end_beats: f64,
    pub removed: bool,
    original: Option<(String, f64, f64)>,
}

impl RegionDraft {
    fn changed(&self) -> bool {
        self.removed
            || self.id == 0
            || self.original.as_ref().is_none_or(|(name, start, end)| {
                *name != self.name
                    || (*start - self.start_beats).abs() > 1.0e-9
                    || (*end - self.end_beats).abs() > 1.0e-9
            })
    }
}

pub(super) struct RegionEditor {
    pub drafts: Vec<RegionDraft>,
    /// Selected draft indices, so a new (id 0) draft can be selected too.
    pub selected: std::collections::BTreeSet<usize>,
    /// Keyboard cursor, moved with the arrow keys.
    pub cursor: usize,
    /// True while the delete confirmation is showing.
    pub confirm_delete: bool,
    pub locate: Option<f64>,
}

impl RegionEditor {
    pub(super) fn from_project(project: &Project) -> Self {
        Self {
            drafts: project
                .regions
                .iter()
                .map(|region| RegionDraft {
                    id: region.id,
                    name: region.name.clone(),
                    start_beats: region.start_beats,
                    end_beats: region.end_beats,
                    removed: false,
                    original: Some((region.name.clone(), region.start_beats, region.end_beats)),
                })
                .collect(),
            selected: std::collections::BTreeSet::new(),
            cursor: 0,
            confirm_delete: false,
            locate: None,
        }
    }

    /// Selects one row, extending the selection when `additive`.
    pub(super) fn toggle(&mut self, index: usize, additive: bool) {
        if index >= self.drafts.len() {
            return;
        }
        if additive {
            if !self.selected.insert(index) {
                self.selected.remove(&index);
            }
        } else {
            self.selected.clear();
            self.selected.insert(index);
        }
        self.cursor = index;
        self.confirm_delete = false;
    }

    pub(super) fn select_all(&mut self) {
        self.selected = (0..self.drafts.len()).collect();
        self.confirm_delete = false;
    }

    pub(super) fn clear_selection(&mut self) {
        self.selected.clear();
        self.confirm_delete = false;
    }

    /// Moves the keyboard cursor within the rows.
    pub(super) fn move_cursor(&mut self, delta: isize, extend: bool) {
        if self.drafts.is_empty() {
            return;
        }
        let next = (self.cursor as isize + delta).clamp(0, self.drafts.len() as isize - 1) as usize;
        self.cursor = next;
        if extend {
            self.selected.insert(next);
        }
        self.confirm_delete = false;
    }

    /// Marks every selected draft for removal and reports the count.
    pub(super) fn mark_selected_removed(&mut self) -> usize {
        let mut removed = 0;
        for index in std::mem::take(&mut self.selected) {
            if let Some(draft) = self.drafts.get_mut(index)
                && !draft.removed
            {
                draft.removed = true;
                removed += 1;
            }
        }
        self.confirm_delete = false;
        removed
    }
}

impl OpenTrackApp {
    pub(super) fn open_region_editor(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to edit regions".to_owned();
            return;
        }
        self.region_editor = Some(RegionEditor::from_project(&self.data.project));
    }

    pub(super) fn apply_region_editor(&mut self, editor: &RegionEditor) {
        let snap = self
            .arrange_view_state
            .snap_grid_beats(self.data.project.beats_per_bar);
        let snapped = |value: f64| {
            snap.map_or(value, |grid| (value / grid).round() * grid)
                .clamp(0.0, 1_000_000.0)
        };
        let mut candidate = self.data.project.clone();
        for draft in &editor.drafts {
            if draft.removed {
                if draft.id != 0 {
                    candidate.remove_region(draft.id);
                }
                continue;
            }
            let name = draft.name.trim();
            if name.is_empty() {
                self.status_message = "A region name cannot be empty".to_owned();
                return;
            }
            let start = snapped(draft.start_beats);
            let end = snapped(draft.end_beats);
            if draft.id == 0 {
                if candidate.add_region(start, end, name.to_owned()).is_none() {
                    self.status_message =
                        "A new region needs a positive range and a name".to_owned();
                    return;
                }
            } else {
                candidate.rename_region(draft.id, name.to_owned());
                if !candidate.set_region_range(draft.id, start, end) {
                    self.status_message = "A region range must end after it starts".to_owned();
                    return;
                }
            }
        }
        if candidate.validate().is_err() {
            self.status_message = "The region changes are invalid; nothing changed".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot edit regions: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Edit Regions");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message =
            "Regions updated; selection and audio events are unchanged".to_owned();
    }

    pub(super) fn show_region_editor(&mut self, ctx: &egui::Context) {
        let Some(mut editor) = self.region_editor.take() else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let mut add = false;
        let mut from_locators = false;
        let response = egui::Window::new("Regions")
            .id(Id::new("project-regions"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(560.0, 340.0))
            .open(&mut open)
            .show(ctx, |ui| {
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    editor.move_cursor(1, ui.input(|i| i.modifiers.shift));
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    editor.move_cursor(-1, ui.input(|i| i.modifiers.shift));
                }
                if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A)) {
                    editor.select_all();
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    editor.clear_selection();
                }
                if ui.input(|i| i.key_pressed(egui::Key::Space))
                    && editor.selected.is_empty()
                    && !editor.drafts.is_empty()
                {
                    editor.toggle(editor.cursor, false);
                }
                if ui.input(|i| i.key_pressed(egui::Key::Delete)) && !editor.selected.is_empty() {
                    editor.confirm_delete = true;
                }
                ui.horizontal(|ui| {
                    if ui.button("Add Region").clicked() {
                        add = true;
                    }
                    if ui.button("From Locators").clicked() {
                        from_locators = true;
                    }
                    if ui
                        .add_enabled(
                            !editor.drafts.is_empty(),
                            egui::Button::new("Select All"),
                        )
                        .clicked()
                    {
                        editor.select_all();
                    }
                    if ui
                        .add_enabled(
                            !editor.selected.is_empty(),
                            egui::Button::new("Delete Selected"),
                        )
                        .clicked()
                    {
                        editor.confirm_delete = true;
                    }
                    ui.label(
                        egui::RichText::new(format!("{} region(s)", editor.drafts.len()))
                            .small()
                            .color(theme::palette(ui).muted),
                    );
                });
                ui.label(
                    egui::RichText::new(
                        "Regions are navigation ranges. Selecting one here does not select audio events; Apply commits all edits as one undo step.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.separator();
                let mut locate = None;
                let mut toggle = None;
                egui::ScrollArea::vertical()
                    .id_salt("project-regions-list")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for (index, draft) in editor.drafts.iter_mut().enumerate() {
                            let selected = editor.selected.contains(&index);
                            ui.horizontal(|ui| {
                                let additive =
                                    ui.input(|i| i.modifiers.command || i.modifiers.shift);
                                if ui
                                    .selectable_label(selected, "◆")
                                    .on_hover_text("Select region (Ctrl/Shift adds)")
                                    .clicked()
                                {
                                    toggle = Some((index, additive));
                                }
                                if ui
                                    .add_enabled(
                                        !draft.removed,
                                        egui::Button::new(if draft.removed { "Undo" } else { "✕" }),
                                    )
                                    .on_hover_text(if draft.removed {
                                        "Keep region"
                                    } else {
                                        "Remove region on Apply"
                                    })
                                    .clicked()
                                {
                                    draft.removed = !draft.removed;
                                }
                                ui.add(
                                    egui::TextEdit::singleline(&mut draft.name)
                                        .desired_width(140.0),
                                );
                                ui.label("start");
                                ui.add(
                                    egui::DragValue::new(&mut draft.start_beats)
                                        .range(0.0..=1_000_000.0)
                                        .speed(0.01),
                                );
                                ui.label("end");
                                ui.add(
                                    egui::DragValue::new(&mut draft.end_beats)
                                        .range(0.0..=1_000_000.0)
                                        .speed(0.01),
                                );
                                if ui.small_button("→").on_hover_text("Locate").clicked() {
                                    locate = Some(draft.start_beats);
                                }
                                if draft.changed() {
                                    ui.label(
                                        egui::RichText::new("edited")
                                            .small()
                                            .color(theme::palette(ui).muted),
                                    );
                                }
                            });
                        }
                    });
                if let Some((index, additive)) = toggle {
                    editor.toggle(index, additive);
                }
                if let Some(beat) = locate {
                    editor.locate = Some(beat);
                }
                ui.separator();
                if editor.confirm_delete {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Delete {} selected region(s)?",
                            editor.selected.len()
                        ));
                        if ui.button("Delete").clicked() {
                            let removed = editor.mark_selected_removed();
                            self.status_message = format!(
                                "Marked {removed} region(s) for removal; Apply commits it"
                            );
                        }
                        if ui.button("Keep").clicked() {
                            editor.confirm_delete = false;
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        super::dismiss_window_on_outside_click(
            ctx,
            Id::new("project-regions"),
            &mut open,
            response.as_ref(),
        );
        if add {
            let number = editor.drafts.len() + 1;
            editor.drafts.push(RegionDraft {
                id: 0,
                name: format!("Region {number}"),
                start_beats: 0.0,
                end_beats: 4.0,
                removed: false,
                original: None,
            });
        }
        if from_locators {
            let number = editor.drafts.len() + 1;
            editor.drafts.push(RegionDraft {
                id: 0,
                name: format!("Region {number}"),
                start_beats: self.data.project.loop_start_beats,
                end_beats: self.data.project.loop_end_beats,
                removed: false,
                original: None,
            });
        }
        if apply {
            self.apply_region_editor(&editor);
        } else if !cancel && open {
            if let Some(beat) = editor.locate.take() {
                self.seek_to_beats(beat);
            }
            self.region_editor = Some(editor);
        }
    }
}

#[cfg(test)]
mod polish_tests {
    use super::*;

    fn app_with_regions() -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_region(0.0, 4.0, "Intro").unwrap();
        app.data.project.add_region(4.0, 8.0, "Verse").unwrap();
        app.data.project.add_region(8.0, 12.0, "Chorus").unwrap();
        // An audio event selection that regions must never disturb.
        app.data.project.add_track(TrackKind::Audio);
        let clip = app
            .data
            .project
            .add_audio_clip(0, PathBuf::from("take.wav"), 0.0, 1.0)
            .unwrap();
        app.arrange_view_state.audio_events.select_many([clip]);
        app
    }

    #[test]
    fn multi_select_and_keyboard_navigation_are_separate_from_event_selection() {
        let mut app = app_with_regions();
        let events_before = app.arrange_view_state.audio_events.selected_ids();
        app.open_region_editor();
        let editor = app.region_editor.as_mut().expect("editor");
        assert_eq!(editor.drafts.len(), 3);

        // Plain click selects one row; additive clicks extend the set.
        editor.toggle(0, false);
        assert_eq!(editor.selected.iter().copied().collect::<Vec<_>>(), vec![0]);
        editor.toggle(2, true);
        assert_eq!(
            editor.selected.iter().copied().collect::<Vec<_>>(),
            vec![0, 2]
        );
        editor.select_all();
        assert_eq!(editor.selected.len(), 3);
        editor.clear_selection();
        assert!(editor.selected.is_empty());

        // Arrow keys move the cursor and can extend the selection.
        editor.cursor = 0;
        editor.move_cursor(1, false);
        assert_eq!(editor.cursor, 1);
        assert!(editor.selected.is_empty());
        editor.move_cursor(1, true);
        assert_eq!(editor.cursor, 2);
        assert!(editor.selected.contains(&2));
        editor.move_cursor(10, false);
        assert_eq!(editor.cursor, 2, "the cursor stops at the last row");

        // Deleting asks for confirmation first; only confirming marks drafts.
        editor.toggle(1, true);
        assert!(editor.selected.contains(&1));
        editor.confirm_delete = true;
        let removed = editor.mark_selected_removed();
        assert_eq!(removed, 2);
        assert!(editor.drafts[1].removed && editor.drafts[2].removed);
        assert!(!editor.drafts[0].removed);
        assert!(editor.selected.is_empty(), "the selection is consumed");

        // Applying removes exactly the marked regions and leaves events alone.
        let editor = app.region_editor.take().expect("editor");
        app.apply_region_editor(&editor);
        let names = app
            .data
            .project
            .regions
            .iter()
            .map(|region| region.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Intro"]);
        assert_eq!(
            app.arrange_view_state.audio_events.selected_ids(),
            events_before,
            "region selection never touches audio events"
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Edit Regions");
    }

    #[test]
    fn the_confirmation_prompt_appears_in_the_window() {
        let ctx = egui::Context::default();
        let mut app = app_with_regions();
        app.region_editor = Some(RegionEditor {
            confirm_delete: true,
            selected: [0_usize, 1].into_iter().collect(),
            ..RegionEditor::from_project(&app.data.project)
        });
        let mut texts = Vec::new();
        for frame in 0..2 {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0))),
                    time: Some(frame as f64 * 0.05),
                    ..Default::default()
                },
                |_| app.show_region_editor(&ctx),
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
        assert!(joined.contains("Delete 2 selected region(s)?"), "{joined}");
        assert!(joined.contains("Keep"), "{joined}");
    }
}
