//! Channel editor spectrum state. FFT work runs here/on the UI thread, never in callbacks.
use super::*;
use opentrack_audio::{SpectrumFrame, SpectrumStage, StereoMeter};
#[derive(Clone)]
pub(super) struct Choice {
    pub id: u64,
    pub name: String,
    pub equalizer: opentrack_core::project::ChannelEqualizer,
}
#[derive(Default)]
pub(super) struct State {
    pub stage: SpectrumStage,
    pub release_ms: u32,
    pub compare: Option<u64>,
    owner: Option<u64>,
    pub choices: Vec<Choice>,
    active: Vec<(u64, Arc<StereoMeter>)>,
    frames: HashMap<(u64, SpectrumStage), SpectrumFrame>,
    seen: HashMap<u64, (u64, Instant)>,
    last_poll: Option<Instant>,
    default_rate: u32,
}
impl State {
    pub fn close(&mut self) {
        for (_, tap) in self.active.drain(..) {
            tap.enable_spectrum(false);
        }
        self.owner = None;
        self.compare = None;
        self.stage = SpectrumStage::Post;
        self.frames.clear();
        self.seen.clear();
        self.last_poll = None;
    }
    pub fn update(
        &mut self,
        owner: Option<u64>,
        choices: Vec<Choice>,
        available: HashMap<u64, Arc<StereoMeter>>,
        rate: u32,
    ) {
        if self.owner != owner {
            self.close();
            self.owner = owner;
        }
        self.choices = choices;
        self.default_rate = rate;
        if self
            .compare
            .is_some_and(|id| Some(id) == owner || !self.choices.iter().any(|c| c.id == id))
        {
            self.compare = None;
        }
        let wanted = owner
            .into_iter()
            .chain(self.compare)
            .filter_map(|id| available.get(&id).map(|tap| (id, Arc::clone(tap))))
            .collect::<Vec<_>>();
        for (id, tap) in &self.active {
            if !wanted
                .iter()
                .any(|(other, current)| other == id && Arc::ptr_eq(tap, current))
            {
                tap.enable_spectrum(false);
                self.frames.retain(|(key, _), _| key != id);
                self.seen.remove(id);
            }
        }
        for (id, tap) in &wanted {
            if !self
                .active
                .iter()
                .any(|(old, previous)| old == id && Arc::ptr_eq(previous, tap))
            {
                tap.enable_spectrum(true);
                self.last_poll = None;
            }
        }
        self.active = wanted;
        let now = Instant::now();
        if self
            .last_poll
            .is_some_and(|last| now.duration_since(last).as_millis() < 30)
        {
            return;
        }
        let dt = self
            .last_poll
            .map(|last| now.duration_since(last).as_secs_f32())
            .unwrap_or(0.03);
        self.last_poll = Some(now);
        for (id, tap) in &self.active {
            if let Some(mut frame) = tap.spectrum_frame(self.stage) {
                let seen = self.seen.entry(*id).or_insert((frame.sequence, now));
                if seen.0 != frame.sequence {
                    *seen = (frame.sequence, now);
                }
                if now.duration_since(seen.1).as_millis() < 250 {
                    if let Some(old) = self.frames.get(&(*id, self.stage)) {
                        metering::smooth_spectrum(
                            &old.bins_dbfs,
                            &mut frame.bins_dbfs,
                            dt,
                            self.release_ms,
                        );
                    }
                    self.frames.insert((*id, self.stage), frame);
                } else {
                    self.frames.remove(&(*id, self.stage));
                }
            } else {
                self.frames.remove(&(*id, self.stage));
            }
        }
    }
    pub fn controls(&mut self, ui: &mut egui::Ui, r: Rect) {
        ui.painter()
            .rect_filled(r, 0.0, theme::palette(ui).surface_raised);
        let old_stage = self.stage;
        let old_compare = self.compare;
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(r.shrink(2.0))
                .id_salt("spectrum-controls"),
            |ui| {
                ui.style_mut().override_font_id = Some(FontId::proportional(10.0));
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("analysis-stage")
                        .width(118.0)
                        .selected_text(if self.stage == SpectrumStage::Pre {
                            "Pre Inserts/EQ"
                        } else {
                            "Post Inserts/EQ"
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.stage,
                                SpectrumStage::Pre,
                                "Pre Inserts/EQ",
                            );
                            ui.selectable_value(
                                &mut self.stage,
                                SpectrumStage::Post,
                                "Post Inserts/EQ",
                            );
                        });
                    ui.label("Compare");
                    let name = self
                        .compare
                        .and_then(|id| self.choices.iter().find(|c| c.id == id))
                        .map(|c| c.name.as_str())
                        .unwrap_or("None");
                    egui::ComboBox::from_id_salt("analysis-reference")
                        .width(135.0)
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.compare, None, "None");
                            for channel in &self.choices {
                                if Some(channel.id) != self.owner {
                                    ui.selectable_value(
                                        &mut self.compare,
                                        Some(channel.id),
                                        &channel.name,
                                    );
                                }
                            }
                        });
                });
            },
        );
        if old_stage != self.stage {
            self.frames.clear();
            self.last_poll = None;
        }
        if old_compare != self.compare {
            self.last_poll = None;
        }
    }
    pub fn rate(&self, id: Option<u64>) -> u32 {
        id.or(self.owner)
            .and_then(|id| self.frames.get(&(id, self.stage)))
            .map(|f| f.sample_rate)
            .unwrap_or(self.default_rate.max(1))
    }
    pub fn paint(&self, ui: &egui::Ui, plot: Rect, own: Color32) {
        let painter = ui.painter().with_clip_rect(plot);
        for (id, color, fill) in [
            (self.owner, own, true),
            (self.compare, Color32::from_rgb(245, 155, 55), false),
        ] {
            let Some(frame) = id.and_then(|id| self.frames.get(&(id, self.stage))) else {
                continue;
            };
            // Aggregate bins per display column so narrow peaks survive zoom and resizing.
            let width = plot.width().ceil().clamp(2.0, 2048.0) as usize;
            let mut levels = vec![-120.0_f32; width];
            for (i, db) in frame.bins_dbfs.iter().enumerate().skip(1) {
                let hz = frame.frequency_hz(i);
                if !(20.0..=20000.0).contains(&hz) {
                    continue;
                }
                let column =
                    ((hz / 20.0).ln() / 1000.0_f32.ln() * (width - 1) as f32).round() as usize;
                levels[column] = levels[column].max(*db);
            }
            let mut points = Vec::new();
            for (i, db) in levels.iter().enumerate().filter(|(_, db)| **db > -119.0) {
                points.push(Pos2::new(
                    plot.left() + i as f32 / (width - 1) as f32 * plot.width(),
                    plot.bottom() - ((*db + 90.0) / 96.0).clamp(0.0, 1.0) * plot.height(),
                ));
            }
            if points.len() > 1 {
                if fill {
                    let mut mesh = egui::Mesh::default();
                    for (i, point) in points.iter().enumerate() {
                        mesh.colored_vertex(*point, color.gamma_multiply(0.18));
                        mesh.colored_vertex(
                            Pos2::new(point.x, plot.bottom()),
                            Color32::TRANSPARENT,
                        );
                        if i > 0 {
                            let n = (i * 2) as u32;
                            mesh.add_triangle(n - 2, n - 1, n);
                            mesh.add_triangle(n - 1, n + 1, n);
                        }
                    }
                    painter.add(egui::Shape::mesh(mesh));
                }
                painter.add(egui::Shape::line(
                    points,
                    Stroke::new(
                        if fill { 1.0 } else { 1.3 },
                        color.gamma_multiply(if fill { 0.65 } else { 0.9 }),
                    ),
                ));
            }
        }
        if let Some(reference) = self
            .compare
            .and_then(|id| self.choices.iter().find(|c| c.id == id))
        {
            let rate = self.rate(Some(reference.id));
            let curve = (0..=160)
                .map(|i| {
                    let hz = 20.0 * 1000.0_f32.powf(i as f32 / 160.0);
                    let db = opentrack_audio::channel_equalizer_response_db(
                        &reference.equalizer,
                        hz,
                        rate,
                    )
                    .clamp(-24.0, 24.0);
                    Pos2::new(
                        plot.left() + plot.width() * i as f32 / 160.0,
                        plot.center().y - db / 48.0 * plot.height(),
                    )
                })
                .collect();
            painter.add(egui::Shape::line(
                curve,
                Stroke::new(1.1, Color32::from_rgb(245, 155, 55)),
            ));
        }
        for db in [0, -30, -60, -90] {
            let y = plot.bottom() - (db as f32 + 90.0) / 96.0 * plot.height();
            ui.painter().text(
                Pos2::new(plot.right() + 2.0, y),
                Align2::LEFT_CENTER,
                format!("{db}"),
                FontId::proportional(8.0),
                theme::palette(ui).muted,
            );
        }
        ui.painter().text(
            plot.right_top() + Vec2::new(2.0, -10.0),
            Align2::LEFT_TOP,
            "dBFS",
            FontId::proportional(8.0),
            theme::palette(ui).muted,
        );
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changing_channel_or_closing_resets_analysis_and_releases_taps() {
        let a = Arc::new(StereoMeter::default());
        let b = Arc::new(StereoMeter::default());
        let choices = vec![
            Choice {
                id: 1,
                name: "A".into(),
                equalizer: Default::default(),
            },
            Choice {
                id: 2,
                name: "B".into(),
                equalizer: Default::default(),
            },
        ];
        let available = HashMap::from([(1, a.clone()), (2, b.clone())]);
        let mut state = State::default();
        state.update(Some(1), choices.clone(), available.clone(), 48000);
        state.stage = SpectrumStage::Pre;
        state.compare = Some(2);
        state.update(Some(1), choices.clone(), available.clone(), 48000);
        assert!(a.spectrum_enabled() && b.spectrum_enabled());
        state.update(Some(2), choices, available, 44100);
        assert_eq!(state.stage, SpectrumStage::Post);
        assert_eq!(state.compare, None);
        assert!(!a.spectrum_enabled());
        assert!(b.spectrum_enabled());
        assert_eq!(state.rate(None), 44100);
        state.stage = SpectrumStage::Pre;
        state.compare = Some(1);
        state.close();
        assert!(!b.spectrum_enabled());
        assert_eq!(state.stage, SpectrumStage::Post);
        assert_eq!(state.compare, None);
    }
}
