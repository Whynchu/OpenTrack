//! Arranger editor: section ranges and an ordered sequence over stable IDs.
//!
//! The draft is transient; Apply validates the whole arranger and commits one
//! undoable content step. It never moves or copies source clips. Add Arranger
//! stays disabled until the audible preview exists in a later step.

use super::*;
use opentrack_core::arranger::Arranger;

pub(super) struct ArrangerEditor {
    pub draft: Arranger,
    pub selected_section: Option<u64>,
    pub region_choice: usize,
    pub sequence_section: Option<u64>,
}

impl ArrangerEditor {
    fn from_project(project: &Project) -> Self {
        Self {
            draft: project.arranger.clone(),
            selected_section: None,
            region_choice: 0,
            sequence_section: None,
        }
    }
}

impl OpenTrackApp {
    pub(super) fn open_arranger_editor(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project to edit the arranger".to_owned();
            return;
        }
        self.arranger_editor = Some(ArrangerEditor::from_project(&self.data.project));
    }

    pub(super) fn apply_arranger_editor(&mut self, editor: &ArrangerEditor) {
        let mut candidate = self.data.project.clone();
        candidate.arranger = editor.draft.clone();
        if candidate.validate().is_err() {
            self.status_message =
                "The arranger changes are invalid or reference a missing section; nothing changed"
                    .to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message =
                "Cannot edit the arranger: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Edit Arranger");
        self.data.project = candidate;
        self.project_history.commit_after_frame = true;
        self.mark_modified();
        self.status_message =
            "Arranger updated; source clips and ordinary timeline playback are unchanged"
                .to_owned();
    }

    pub(super) fn show_arranger_editor(&mut self, ctx: &egui::Context) {
        let Some(mut editor) = self.arranger_editor.take() else {
            return;
        };
        let regions = self.data.project.regions.clone();
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        let response = egui::Window::new("Arranger")
            .id(Id::new("project-arranger"))
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(true)
            .default_size(Vec2::new(720.0, 460.0))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Sections reference source timeline ranges. The sequence plays section IDs in order; source clips are never moved. Ordinary timeline playback is preserved.",
                    )
                    .small()
                    .color(theme::palette(ui).muted),
                );
                ui.separator();
                ui.heading("Sections (source ranges)");
                ui.horizontal(|ui| {
                    if ui.button("Add from Locators").clicked() {
                        let number = editor.draft.sections.len() + 1;
                        if let Some(id) = editor.draft.add_section(
                            format!("Section {number}"),
                            self.data.project.loop_start_beats,
                            self.data.project.loop_end_beats,
                        ) {
                            editor.selected_section = Some(id);
                        }
                    }
                    let selected_region = regions.get(editor.region_choice.min(
                        regions.len().saturating_sub(1),
                    ));
                    egui::ComboBox::from_id_salt("arranger-region-choice")
                        .width(150.0)
                        .selected_text(
                            selected_region
                                .map(|region| region.name.as_str())
                                .unwrap_or("No regions"),
                        )
                        .show_ui(ui, |ui| {
                            for (index, region) in regions.iter().enumerate() {
                                ui.selectable_value(
                                    &mut editor.region_choice,
                                    index,
                                    region.name.as_str(),
                                );
                            }
                        });
                    if ui
                        .add_enabled(selected_region.is_some(), egui::Button::new("Add from Region"))
                        .clicked()
                        && let Some(region) = selected_region
                    {
                        if let Some(id) = editor.draft.add_section(
                            region.name.clone(),
                            region.start_beats,
                            region.end_beats,
                        ) {
                            editor.selected_section = Some(id);
                        }
                    }
                });
                let mut remove_section = None;
                egui::ScrollArea::vertical()
                    .id_salt("arranger-sections")
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for section in &mut editor.draft.sections {
                            ui.horizontal(|ui| {
                                let selected = editor.selected_section == Some(section.id);
                                if ui
                                    .selectable_label(selected, "◆")
                                    .on_hover_text("Select section")
                                    .clicked()
                                {
                                    editor.selected_section = Some(section.id);
                                    editor.sequence_section = Some(section.id);
                                }
                                ui.add(
                                    egui::TextEdit::singleline(&mut section.name)
                                        .desired_width(130.0),
                                );
                                ui.label("source");
                                ui.add(
                                    egui::DragValue::new(&mut section.start_beats)
                                        .range(0.0..=opentrack_core::arranger::MAX_ARRANGER_BEATS)
                                        .speed(0.01),
                                );
                                ui.label("to");
                                ui.add(
                                    egui::DragValue::new(&mut section.end_beats)
                                        .range(0.0..=opentrack_core::arranger::MAX_ARRANGER_BEATS)
                                        .speed(0.01),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "({:.3} beats)",
                                        (section.end_beats - section.start_beats).max(0.0)
                                    ))
                                    .small()
                                    .color(theme::palette(ui).muted),
                                );
                                if ui.small_button("✕").on_hover_text("Remove section").clicked() {
                                    remove_section = Some(section.id);
                                }
                            });
                        }
                    });
                if let Some(id) = remove_section {
                    editor.draft.remove_section(id);
                    if editor.selected_section == Some(id) {
                        editor.selected_section = None;
                    }
                    if editor.sequence_section == Some(id) {
                        editor.sequence_section = None;
                    }
                }
                ui.separator();
                ui.heading("Sequence (play order)");
                ui.horizontal(|ui| {
                    let selected = editor
                        .sequence_section
                        .and_then(|id| editor.draft.section(id))
                        .map(|section| section.name.as_str())
                        .unwrap_or("Select a section");
                    egui::ComboBox::from_id_salt("arranger-sequence-section")
                        .width(150.0)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for section in &editor.draft.sections {
                                ui.selectable_value(
                                    &mut editor.sequence_section,
                                    Some(section.id),
                                    section.name.as_str(),
                                );
                            }
                        });
                    if ui
                        .add_enabled(
                            editor.sequence_section.is_some(),
                            egui::Button::new("Append Section"),
                        )
                        .clicked()
                        && let Some(id) = editor.sequence_section
                    {
                        editor.draft.append_sequence(id, 1);
                    }
                });
                let mut sequence_move: Option<(usize, usize)> = None;
                let mut sequence_remove = None;
                let mut sequence_position = 0.0;
                let section_info = editor
                    .draft
                    .sections
                    .iter()
                    .map(|section| {
                        (
                            section.id,
                            section.name.clone(),
                            (section.end_beats - section.start_beats).max(0.0),
                        )
                    })
                    .collect::<Vec<_>>();
                let sequence_len = editor.draft.sequence.len();
                egui::ScrollArea::vertical()
                    .id_salt("arranger-sequence")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        if sequence_len == 0 {
                            ui.label(
                                "The sequence is empty. The ordinary timeline remains the playback source.",
                            );
                        }
                        for (index, item) in editor.draft.sequence.iter_mut().enumerate() {
                            let Some((_, name, length)) =
                                section_info.iter().find(|(id, _, _)| *id == item.section_id)
                            else {
                                continue;
                            };
                            let start = sequence_position;
                            sequence_position += length * f64::from(item.repeats.max(1));
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("seq {start:.3}"))
                                        .small()
                                        .color(theme::palette(ui).muted),
                                );
                                ui.label(format!("{}. {}", index + 1, name));
                                ui.label("repeats");
                                ui.add(egui::DragValue::new(&mut item.repeats).range(1..=1024));
                                if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                                    sequence_move = Some((index, index - 1));
                                }
                                if ui
                                    .add_enabled(
                                        index + 1 < sequence_len,
                                        egui::Button::new("↓"),
                                    )
                                    .clicked()
                                {
                                    sequence_move = Some((index, index + 1));
                                }
                                if ui.small_button("✕").clicked() {
                                    sequence_remove = Some(index);
                                }
                            });
                        }
                    });
                if let Some((from, to)) = sequence_move {
                    editor.draft.move_sequence_item(from, to);
                }
                if let Some(index) = sequence_remove {
                    editor.draft.remove_sequence_item(index);
                }
                ui.separator();
                if let Some(duration) = editor.draft.expanded_duration_beats() {
                    ui.label(format!("Sequence duration: {duration:.3} beats"));
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
            Id::new("project-arranger"),
            &mut open,
            response.as_ref(),
        );
        if apply {
            self.apply_arranger_editor(&editor);
        } else if !cancel && open {
            self.arranger_editor = Some(editor);
        }
    }
}
