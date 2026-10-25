//! Application preferences; only connected settings are editable.
use super::*;
pub(super) const PAGES: &[(&str, &str, &str)] = &[
    ("Appearance", "appearance", "Interface appearance"),
    (
        "  General",
        "appearance-general",
        "Saturation, brightness and button brightness",
    ),
    (
        "  Meters",
        "appearance-meters",
        "Meter gradient colors and thresholds",
    ),
    (
        "  Work Area",
        "appearance-work-area",
        "Work-area, cycle and grid intensities",
    ),
    (
        "Editing",
        "editing",
        "Track time type, selection, overlaps, locking, zoom, drag delay and automation-follow behavior",
    ),
    (
        "  Audio",
        "editing-audio",
        "Stretch algorithm, import/shared-clip policy, event volume/fades, hitpoints and zero crossing",
    ),
    (
        "  Controls",
        "editing-controls",
        "Value-box/time control, knob and slider modes",
    ),
    (
        "  MIDI",
        "editing-midi",
        "Controller selection, legato overlap and split policies",
    ),
    (
        "  Project & Mixer",
        "editing-project-mixer",
        "Selection synchronization, auto track colors, record/solo selection, folding and track enlargement",
    ),
    (
        "  Tool Modifiers",
        "editing-tool-modifiers",
        "Per-tool modifier bindings",
    ),
    (
        "  Tools",
        "editing-tools",
        "Extra tool info, zoom behavior, right-click toolbox and crosshair",
    ),
    (
        "Event Display",
        "event-display",
        "Names, transparency, small-track data and event background color",
    ),
    (
        "  Audio",
        "event-display-audio",
        "Wave-image style/interpolation, volume curves, fade handles/lines and waveforms",
    ),
    (
        "  MIDI",
        "event-display-midi",
        "Default editor, part-data display, controllers, drum-map editing and note names",
    ),
    ("  Rulers", "event-display-rulers", "Strong ruler colors"),
    (
        "General",
        "general",
        "Language, auto-save interval/backups, tips, undo limit and startup policy",
    ),
    (
        "MIDI",
        "midi",
        "Thru, stop reset, note lengths, chase, display resolution and feedback",
    ),
    (
        "  MIDI File",
        "midi-file",
        "MIDI import/export policies and resolution",
    ),
    (
        "  MIDI Filter",
        "midi-filter",
        "Record/thru event filters, channels and controller filters",
    ),
    (
        "MediaBay",
        "mediabay",
        "Result limits, viewer editing, folder scanning and extensions",
    ),
    ("Record", "record", "Recording preferences"),
    (
        "  Audio",
        "record-audio",
        "Pre-record duration, live waveform generation and audio cycle-record mode",
    ),
    (
        "    Broadcast Wave",
        "record-broadcast-wave",
        "Metadata preferences; page not captured, contents unknown",
    ),
    (
        "  MIDI",
        "record-midi",
        "Bar snapping, editor solo-record, catch range, retrospective buffer and replace mode",
    ),
    (
        "Transport",
        "transport",
        "Locating/zooming, cursor width, stop/punch/wind policies, subframes and empty-space locate",
    ),
    (
        "  Scrub",
        "transport-scrub",
        "Scrub speed/volume and CPU-saving mode",
    ),
    (
        "VST",
        "vst",
        "Channel defaults, mute/automation, delay compensation, panner and monitoring policies",
    ),
    (
        "  Plug-ins",
        "vst-plugins",
        "Effect-removal warnings, editor opening/topmost, instrument MIDI creation, silence suspension and sorting",
    ),
    (
        "  Metering",
        "vst-metering",
        "Working meter and spectrum timing",
    ),
    (
        "  Control Room",
        "vst-control-room",
        "Talkback, phones, dim, indicators, solo, downmix, metronome and reference level",
    ),
    (
        "VariAudio",
        "variaudio",
        "Warnings for sample-data and offline-process edits",
    ),
];
pub(super) struct Window {
    pub is_open: bool,
    page: &'static str,
    draft: metering::Preferences,
    help: bool,
}
impl Default for Window {
    fn default() -> Self {
        Self {
            is_open: false,
            page: "vst-metering",
            draft: Default::default(),
            help: false,
        }
    }
}
impl Window {
    pub fn open(&mut self, settings: &metering::Preferences) {
        self.draft = settings.clone();
        self.is_open = true;
    }
    pub fn show(&mut self, ctx: &egui::Context, settings: &mut metering::Preferences) {
        if !self.is_open {
            return;
        }
        let mut open = true;
        let mut done = false;
        egui::Window::new("Preferences").id(Id::new("application-preferences"))
            .open(&mut open).default_size(Vec2::new(970.0,650.0)).min_size(Vec2::new(700.0,450.0)).show(ctx,|ui| {
            let height=(ui.available_height()-80.0).min(ctx.content_rect().height()-180.0).clamp(240.0,500.0);
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical().id_salt("preferences-tree").max_height(height).show(ui,|ui| {
                    ui.set_width(190.0);
                    for (label,id,_) in PAGES {
                        if ui.selectable_label(self.page==*id,*label).clicked() { self.page=id; }
                    }
                });
                let (divider,_) = ui.allocate_exact_size(Vec2::new(1.0,height),Sense::hover());
                ui.painter().rect_filled(divider,0.0,theme::palette(ui).surface_raised);
                ui.vertical(|ui| {
                    ui.set_min_height(height);ui.set_min_width(450.0);
                    let (_,id,description)=PAGES.iter().find(|(_,id,_)| *id==self.page).unwrap();
                    let index=PAGES.iter().position(|(_,key,_)|key==id).unwrap();
                    let label=PAGES[index].0.trim();
                    let title=if PAGES[index].0.starts_with(' ') {
                        let parent=PAGES[..index].iter().rev().find(|(label,_,_)|!label.starts_with(' ')).unwrap().0;
                        format!("{parent} - {label}")
                    } else { label.to_owned() };
                    let rect=ui.allocate_space(Vec2::new(ui.available_width(),26.0)).1;
                    ui.painter().rect_filled(rect,0.0,theme::palette(ui).surface_raised);
                    ui.painter().text(rect.center(),Align2::CENTER_CENTER,title,FontId::proportional(14.0),theme::palette(ui).text);
                    ui.add_space(8.0);
                    if *id=="vst-metering" {
                        ui.add_enabled(false,egui::Checkbox::new(&mut true,"Map Input Bus Metering to Audio Track (in Direct Monitoring)"))
                            .on_disabled_hover_text("Hardware Direct Monitoring is not implemented. Software-monitored track meters already follow their routed audio.");
                        ui.label(egui::RichText::new("Hardware Direct Monitoring is not implemented.").small().color(theme::palette(ui).muted));
                        ui.add_space(4.0);
                        row(ui,&mut self.draft.peak_hold_ms,0..=10000," ms","Meters' Peak Hold Time");
                        row(ui,&mut self.draft.slow_fallback,1.0..=120.0," dB/s","Meters' Slow Fallback");
                        row(ui,&mut self.draft.fast_fallback,1.0..=120.0," dB/s","Meters' Fast Fallback");
                        ui.add_space(16.0);ui.separator();
                        ui.label("Spectrum Analyzer");
                        row(ui,&mut self.draft.spectrum_release_ms,0..=2000," ms","Release smoothing");
                        ui.label(egui::RichText::new("0 ms shows raw FFT frames. Maximum dB readouts remain held until clicked.").small().color(theme::palette(ui).muted));
                        if self.help {
                            ui.add_space(12.0);
                            ui.label("Slow fallback controls the filled level bar. Peak markers hold, then fall at the fast rate. Both use elapsed time and share the same settings across Project, Mixer and Channel Settings. Spectrum attack is immediate; release smoothing steadies the trace. These display settings do not change audio or device latency.");
                        }
                    } else {
                        ui.label("Planned - not implemented in Preferences yet");ui.add_space(8.0);ui.label(*description);
                        ui.label(egui::RichText::new("Recorded from supplied references. Existing fixed behavior elsewhere is not exposed as a working preference here.").small().color(theme::palette(ui).muted));
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Preference Presets");
                ui.add_enabled(false,egui::Button::new("Store"));ui.add_enabled(false,egui::Button::new("Rename"));ui.add_enabled(false,egui::Button::new("Delete"));
                ui.label(egui::RichText::new("Planned").small());
            });
            ui.horizontal(|ui| {
                if ui.button("Help").clicked() {self.help=!self.help;}
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                    if ui.button("Cancel").clicked() {done=true;}
                    if ui.button("OK").clicked() { self.draft.sanitize();*settings=self.draft.clone();done=true; }
                    if ui.button("Apply").clicked() {self.draft.sanitize();*settings=self.draft.clone();}
                    if ui.button("Defaults").clicked() {self.draft=Default::default();}
                });
            });
        });
        self.is_open = open && !done;
    }
}
fn row<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut T,
    range: std::ops::RangeInclusive<T>,
    suffix: &str,
    label: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [95.0, 22.0],
            egui::DragValue::new(value)
                .range(range)
                .suffix(suffix)
                .speed(1.0),
        );
        ui.label(label);
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_tree_excludes_video_and_scores_and_opens_on_metering() {
        assert!(
            PAGES
                .iter()
                .all(|(_, id, _)| !id.contains("video") && !id.contains("score"))
        );
        let ctx = egui::Context::default();
        let mut w = Window::default();
        let mut p = metering::Preferences::default();
        w.open(&p);
        for frame in 0..3 {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 900.0))),
                    ..Default::default()
                },
                |ui| w.show(ui.ctx(), &mut p),
            );
            if frame == 2 {
                assert!(out.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Meters' Peak Hold Time")));
            }
            out.textures_delta.clear();
        }
        assert_eq!(p, metering::Preferences::default());
    }
    #[test]
    fn apply_cancel_ok_and_defaults_have_transactional_global_effects() {
        fn frame(
            ctx: &egui::Context,
            w: &mut Window,
            p: &mut metering::Preferences,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 900.0))),
                    events,
                    ..Default::default()
                },
                |ui| w.show(ui.ctx(), p),
            )
        }
        fn click(ctx: &egui::Context, w: &mut Window, p: &mut metering::Preferences, label: &str) {
            let mut out = frame(ctx, w, p, vec![]);
            let pos = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => {
                        Some(t.pos + t.galley.size() * 0.5)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| {
                    panic!(
                        "missing {label}; open={}; labels={:?}",
                        w.is_open,
                        out.shapes
                            .iter()
                            .filter_map(|s| if let egui::Shape::Text(t) = &s.shape {
                                Some(t.galley.text())
                            } else {
                                None
                            })
                            .collect::<Vec<_>>()
                    )
                });
            out.textures_delta.clear();
            for pressed in [true, false] {
                let mut result = frame(
                    ctx,
                    w,
                    p,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
                result.textures_delta.clear();
            }
        }
        let ctx = egui::Context::default();
        let mut w = Window::default();
        let mut p = metering::Preferences::default();
        w.open(&p);
        for _ in 0..3 {
            let mut out = frame(&ctx, &mut w, &mut p, vec![]);
            out.textures_delta.clear();
        }
        w.draft.slow_fallback = 8.0;
        click(&ctx, &mut w, &mut p, "Apply");
        assert_eq!(p.slow_fallback, 8.0);
        assert!(w.is_open);
        w.draft.slow_fallback = 70.0;
        click(&ctx, &mut w, &mut p, "Cancel");
        assert_eq!(p.slow_fallback, 8.0);
        assert!(!w.is_open);
        w.open(&p);
        assert_eq!(w.draft.slow_fallback, 8.0);
        for _ in 0..2 {
            let mut out = frame(&ctx, &mut w, &mut p, vec![]);
            out.textures_delta.clear();
        }
        click(&ctx, &mut w, &mut p, "Defaults");
        assert_eq!(p.slow_fallback, 8.0);
        assert_eq!(w.draft.slow_fallback, 20.0);
        click(&ctx, &mut w, &mut p, "OK");
        assert_eq!(p.slow_fallback, 20.0);
        assert!(!w.is_open);
    }
    #[test]
    fn metering_preferences_roundtrip_and_invalid_values_are_bounded() {
        let mut p = metering::Preferences {
            peak_hold_ms: 500,
            slow_fallback: 8.0,
            fast_fallback: 60.0,
            spectrum_release_ms: 240,
        };
        let bytes = serde_json::to_string(&p).unwrap();
        let restored: metering::Preferences = serde_json::from_str(&bytes).unwrap();
        assert_eq!(p, restored);
        p.slow_fallback = f32::NAN;
        p.fast_fallback = -1.0;
        p.peak_hold_ms = u32::MAX;
        p.sanitize();
        assert!(p.slow_fallback.is_finite() && p.fast_fallback >= 1.0 && p.peak_hold_ms <= 10000);
    }
}
