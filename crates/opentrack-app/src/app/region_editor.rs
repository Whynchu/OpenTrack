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
    pub selected: Option<u64>,
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
            selected: None,
            locate: None,
        }
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
                ui.horizontal(|ui| {
                    if ui.button("Add Region").clicked() {
                        add = true;
                    }
                    if ui.button("From Locators").clicked() {
                        from_locators = true;
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
                egui::ScrollArea::vertical()
                    .id_salt("project-regions-list")
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for draft in &mut editor.drafts {
                            ui.horizontal(|ui| {
                                let selected = editor.selected == Some(draft.id)
                                    && draft.id != 0;
                                if ui
                                    .selectable_label(selected, "◆")
                                    .on_hover_text("Select region")
                                    .clicked()
                                {
                                    editor.selected = (draft.id != 0).then_some(draft.id);
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
                if let Some(beat) = locate {
                    editor.locate = Some(beat);
                }
                ui.separator();
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
