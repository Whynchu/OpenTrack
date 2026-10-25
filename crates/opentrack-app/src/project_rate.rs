//! Rate changes keep source media immutable, preserving seconds/pitch through engine resampling.
use super::*;
pub(super) struct Change {
    pub rate: u32,
    pub format: RecordingFormat,
    pub mismatched: Vec<(PathBuf, u32)>,
    pub unavailable: Vec<PathBuf>,
}
pub(super) fn inspect(project: &Project, rate: u32, format: RecordingFormat) -> Change {
    let mut change = Change {
        rate,
        format,
        mismatched: Vec::new(),
        unavailable: Vec::new(),
    };
    for path in project.media_paths() {
        match opentrack_audio::audio_file_metadata(&path) {
            Ok(info) if info.sample_rate != rate => {
                change.mismatched.push((path, info.sample_rate))
            }
            Ok(_) => {}
            Err(_) => change.unavailable.push(path),
        }
    }
    change
}
impl OpenTrackApp {
    pub(super) fn request_project_rate(&mut self, rate: u32, format: RecordingFormat) {
        if self.is_recording() {
            return;
        }
        if rate == self.data.project.sample_rate_hz {
            if format != self.data.project.recording_format {
                self.data.project.recording_format = format;
                self.mark_modified();
            }
            return;
        }
        let change = inspect(&self.data.project, rate, format);
        if change.mismatched.is_empty() && change.unavailable.is_empty() {
            self.apply_project_rate(change);
        } else {
            self.project_rate_change = Some(change);
        }
    }
    fn apply_project_rate(&mut self, change: Change) {
        if self.is_recording() {
            self.status_message = "Stop recording before changing the project rate".into();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self.project_history.begin(&self.data.project) {
            self.status_message = "Cannot change rate: undo snapshot unavailable".into();
            return;
        }
        let mut candidate = self.data.project.clone();
        candidate.sample_rate_hz = change.rate;
        candidate.recording_format = change.format;
        if let Err(e) = candidate.validate() {
            self.status_message = e.into();
            self.project_history.pending = None;
            return;
        }
        self.data.project = candidate;
        self.project_history.finish_frame(&self.data.project, true);
        self.rebuild_audio_plan();
        self.mark_modified();
        self.status_message = format!(
            "Project: {} Hz. Existing source files retained; playback resamples without changing timing or pitch.",
            change.rate
        );
    }
    pub(super) fn show_project_rate_change(&mut self, ctx: &egui::Context) {
        let Some(change) = &self.project_rate_change else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        let mut open = true;
        egui::Window::new("Change Project Sample Rate").id(Id::new("project-rate-confirmation")).collapsible(false).resizable(false).default_width(520.0).anchor(Align2::CENTER_CENTER,Vec2::ZERO).open(&mut open).show(ctx,|ui| {
            ui.label(format!("Change project from {} Hz to {} Hz?",self.data.project.sample_rate_hz,change.rate));
            ui.colored_label(theme::WARNING,format!("{} media file(s) have a different sample rate.",change.mismatched.len()));
            egui::ScrollArea::vertical().max_height(140.0).show(ui,|ui|for(path,rate)in &change.mismatched{ui.label(format!("{} - {} Hz",path.file_name().unwrap_or_default().to_string_lossy(),rate));});
            ui.label("Conform playback keeps event timing, duration and pitch. Original files stay unchanged; the audio engine resamples them during playback. The project sample grid and new recordings use the new rate.");
            ui.label("This does not rewrite or replace existing WAV files. Undo restores the previous project rate.");
            if !change.unavailable.is_empty(){ui.colored_label(theme::WARNING,format!("{} media file(s) are unavailable or unsupported; they remain offline.",change.unavailable.len()));}
            ui.horizontal(|ui|{if ui.add_enabled(!self.is_recording(),egui::Button::new("Change Rate / Conform Playback")).clicked(){apply=true;}if ui.button("Cancel").clicked(){cancel=true;}});
        });
        if apply {
            if let Some(change) = self.project_rate_change.take() {
                self.apply_project_rate(change);
            }
        } else if cancel || !open {
            self.project_rate_change = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rate_review_finds_pool_only_media_and_never_rewrites_or_retimes_sources() {
        let (path, _) = sample_editor::tests::fixture_source(1);
        let before = fs::read(&path).unwrap();
        let mut p = Project::default();
        p.add_track(TrackKind::Audio);
        p.add_audio_clip(0, path.clone(), 4.0, 1.0);
        let clips = p.tracks[0].clips.clone();
        let review = inspect(&p, 44100, RecordingFormat::Pcm24);
        assert_eq!(review.mismatched, vec![(path.clone(), 8000)]);
        assert!(review.unavailable.is_empty());
        let mut h = ProjectHistory::default();
        h.begin(&p);
        p.sample_rate_hz = 44100;
        h.finish_frame(&p, true);
        assert_eq!(p.tracks[0].clips, clips);
        let old = h.undo(&p).unwrap();
        assert_eq!(old.sample_rate_hz, 48000);
        assert_eq!(fs::read(&path).unwrap(), before);
        p.remove_track(0);
        assert_eq!(
            inspect(&p, 48000, RecordingFormat::Pcm24).mismatched.len(),
            1
        );
        fs::remove_file(&path).unwrap();
        assert_eq!(
            inspect(&p, 48000, RecordingFormat::Pcm24).unavailable,
            vec![path]
        );
    }
}
