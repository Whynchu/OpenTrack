//! Audio CD import through a CUE sheet.
//!
//! A CUE sheet names one audio file and the track boundaries inside it, which is
//! the shape a CD image takes on disk. The parser is bounded and only handles the
//! commands that define those boundaries; anything else is reported rather than
//! guessed. This is not red-book image support and does not claim CD authoring.

use super::*;

/// Largest cue sheet accepted.
pub(super) const MAX_CUE_BYTES: usize = 64 * 1024;
/// Bound on imported tracks.
pub(super) const MAX_CUE_TRACKS: usize = 99;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CueTrack {
    pub number: u8,
    pub title: Option<String>,
    pub start_seconds: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CueSheet {
    pub file: String,
    pub title: Option<String>,
    pub tracks: Vec<CueTrack>,
    pub unsupported: Vec<String>,
}

impl CueSheet {
    /// Parses a cue sheet, returning its audio file and track starts.
    pub(super) fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_CUE_BYTES {
            return Err("the cue sheet is larger than the 64 KiB import limit".to_owned());
        }
        let mut file = None;
        let mut title = None;
        let mut tracks: Vec<CueTrack> = Vec::new();
        let mut unsupported = Vec::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.splitn(3, char::is_whitespace);
            let command = parts.next().unwrap_or_default().to_ascii_uppercase();
            match command.as_str() {
                "FILE" => {
                    if let Some(name) = parts.next() {
                        // FILE "name.wav" WAVE — keep the quoted or bare name.
                        let name = name.trim();
                        let name = name.strip_prefix('"').unwrap_or(name);
                        let name = name.split('"').next().unwrap_or(name);
                        file = Some(name.to_owned());
                    }
                }
                "TRACK" => {
                    let mut rest = parts;
                    let number = rest
                        .next()
                        .and_then(|value| value.parse::<u8>().ok())
                        .ok_or_else(|| "a TRACK line has no usable number".to_owned())?;
                    let kind = rest.next().unwrap_or_default().to_ascii_uppercase();
                    if kind != "AUDIO" {
                        unsupported.push(format!("track {number} is {kind}, not AUDIO"));
                        continue;
                    }
                    if tracks.len() >= MAX_CUE_TRACKS {
                        return Err(
                            "the cue sheet has more tracks than the import supports".to_owned()
                        );
                    }
                    // A track starts untitled; a following TITLE names it, so a
                    // gap does not inherit the previous track's name.
                    tracks.push(CueTrack {
                        number,
                        title: None,
                        start_seconds: -1.0,
                    });
                }
                "INDEX" => {
                    let Some(track) = tracks.last_mut() else {
                        unsupported.push("INDEX before any TRACK".to_owned());
                        continue;
                    };
                    let index = parts.next().unwrap_or_default();
                    let time = parts.next().unwrap_or_default().trim();
                    if index != "01" {
                        // INDEX 00 is the pregap; only 01 starts the track itself.
                        continue;
                    }
                    track.start_seconds = parse_cue_time(time)?;
                }
                "TITLE" => {
                    let value = parts.next().unwrap_or_default().trim();
                    let value = value.strip_prefix('"').unwrap_or(value);
                    let value = value.split('"').next().unwrap_or(value).trim().to_owned();
                    if tracks.is_empty() {
                        title = Some(value);
                    } else if let Some(track) = tracks.last_mut() {
                        track.title = Some(value);
                    }
                }
                "PERFORMER" | "REM" | "CATALOG" | "FLAGS" | "ISRC" | "PREGAP" | "POSTGAP" => {}
                other => unsupported.push(format!("{other} is not supported")),
            }
        }
        let file = file.ok_or_else(|| "the cue sheet has no FILE line".to_owned())?;
        if tracks.is_empty() {
            return Err("the cue sheet has no AUDIO tracks".to_owned());
        }
        if tracks.iter().any(|track| track.start_seconds < 0.0) {
            return Err("a track has no INDEX 01 position".to_owned());
        }
        tracks.sort_by(|left, right| left.start_seconds.total_cmp(&right.start_seconds));
        unsupported.sort();
        unsupported.dedup();
        Ok(Self {
            file,
            title,
            tracks,
            unsupported,
        })
    }
}

/// Parses `mm:ss:ff` (75 frames per second) into seconds.
fn parse_cue_time(value: &str) -> Result<f64, String> {
    let mut parts = value.split(':');
    let minutes = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .ok_or_else(|| format!("'{value}' is not a cue time"))?;
    let seconds = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .ok_or_else(|| format!("'{value}' is not a cue time"))?;
    let frames = parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .ok_or_else(|| format!("'{value}' is not a cue time"))?;
    if seconds >= 60 || frames >= 75 {
        return Err(format!("'{value}' is not a valid cue time"));
    }
    Ok(f64::from(minutes) * 60.0 + f64::from(seconds) + f64::from(frames) / 75.0)
}

impl OpenTrackApp {
    pub(super) fn import_audio_cd(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project before importing a CD image".to_owned();
            return;
        }
        if self.is_recording() {
            self.status_message = "Stop recording before importing".to_owned();
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter("CUE sheet", &["cue"])
            .set_title("Import Audio CD")
            .pick_file()
        else {
            return;
        };
        self.import_audio_cd_from(&path);
    }

    /// Imports the cue sheet at `path` as one history step.
    pub(super) fn import_audio_cd_from(&mut self, path: &Path) {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                self.status_message = format!("Could not read the cue sheet: {error}");
                return;
            }
        };
        let sheet = match CueSheet::parse(&text) {
            Ok(sheet) => sheet,
            Err(message) => {
                self.status_message = format!("Audio CD import failed: {message}");
                return;
            }
        };
        let audio_path = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&sheet.file);
        let metadata = match opentrack_audio::audio_file_metadata(&audio_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                self.status_message = format!("Audio CD import failed: {error}");
                return;
            }
        };
        let total_seconds = f64::from(metadata.frames) / f64::from(metadata.sample_rate.max(1));
        let mut spans = Vec::with_capacity(sheet.tracks.len());
        for (index, track) in sheet.tracks.iter().enumerate() {
            let end = sheet
                .tracks
                .get(index + 1)
                .map(|next| next.start_seconds)
                .unwrap_or(total_seconds);
            if track.start_seconds >= end || track.start_seconds >= total_seconds {
                self.status_message = format!(
                    "Audio CD import failed: track {} has no audio inside the file",
                    track.number
                );
                return;
            }
            spans.push((track, end));
        }
        let tempo = self.data.project.tempo_bpm.max(1.0);
        let mut candidate = self.data.project.clone();
        candidate.add_track(TrackKind::Audio);
        let track_index = candidate.tracks.len() - 1;
        candidate.tracks[track_index].name = sheet
            .title
            .clone()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Audio CD".to_owned())
            });
        let mut cursor_beats = 0.0;
        let mut created = 0;
        for (track, end) in &spans {
            let duration_seconds = end - track.start_seconds;
            let duration_beats = duration_seconds * tempo / 60.0;
            let Some(clip_id) = candidate.add_audio_clip(
                track_index,
                audio_path.clone(),
                cursor_beats,
                duration_seconds,
            ) else {
                self.status_message = "Audio CD import failed: could not place a track".to_owned();
                return;
            };
            if let Some(clip) = candidate.tracks[track_index]
                .clips
                .iter_mut()
                .find(|clip| clip.id == clip_id)
            {
                clip.source_offset_seconds = track.start_seconds;
                clip.name = match &track.title {
                    Some(title) if !title.trim().is_empty() => title.clone(),
                    _ => format!("Track {:02}", track.number),
                };
            }
            cursor_beats += duration_beats;
            created += 1;
        }
        if candidate.validate().is_err() {
            self.status_message = "The imported tracks would not form a valid project".to_owned();
            return;
        }
        self.project_history.finish_frame(&self.data.project, true);
        if !self
            .project_history
            .begin(&self.data.project, self.modified)
        {
            self.status_message = "Cannot import the CD: undo snapshot is unavailable".to_owned();
            return;
        }
        self.project_history.label_next("Import Audio CD");
        self.data.project = candidate;
        self.data.project.register_media();
        self.project_history.commit_after_frame = true;
        self.rebuild_audio_plan();
        self.mark_modified();
        let mut status = format!("Imported {created} CD track(s) from the cue sheet");
        if !sheet.unsupported.is_empty() {
            status.push_str(&format!("; not imported: {}", sheet.unsupported.join(", ")));
        }
        self.status_message = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHEET: &str = "REM authored fixture\n\
FILE \"album.wav\" WAVE\n\
  TRACK 01 AUDIO\n\
    TITLE \"First\"\n\
    INDEX 01 00:00:00\n\
  TRACK 02 AUDIO\n\
    TITLE \"Second\"\n\
    INDEX 00 00:00:70\n\
    INDEX 01 00:01:00\n\
  TRACK 03 AUDIO\n\
    INDEX 01 00:02:30\n";

    #[test]
    fn cue_sheets_are_parsed_with_bounded_track_starts() {
        let sheet = CueSheet::parse(SHEET).expect("parse");
        assert_eq!(sheet.file, "album.wav");
        assert_eq!(sheet.tracks.len(), 3);
        assert_eq!(sheet.tracks[0].start_seconds, 0.0);
        assert_eq!(sheet.tracks[1].start_seconds, 1.0);
        assert_eq!(sheet.tracks[2].start_seconds, 2.0 + 30.0 / 75.0);
        assert_eq!(sheet.tracks[0].title.as_deref(), Some("First"));
        assert!(sheet.unsupported.is_empty(), "{:?}", sheet.unsupported);

        // Malformed sheets are refused.
        assert!(CueSheet::parse("TRACK 01 AUDIO\n INDEX 01 00:00:00\n").is_err());
        assert!(CueSheet::parse("FILE \"a.wav\" WAVE\n").is_err());
        assert!(CueSheet::parse("FILE \"a.wav\" WAVE\n TRACK 01 AUDIO\n").is_err());
        assert!(
            CueSheet::parse("FILE \"a.wav\" WAVE\n TRACK 01 AUDIO\n INDEX 01 00:99:00\n").is_err()
        );
        // Unsupported commands are reported, not guessed at.
        let sheet = CueSheet::parse(
            "FILE \"a.wav\" WAVE\n TRACK 01 MODE1/2352\n INDEX 01 00:00:00\n TRACK 02 AUDIO\n INDEX 01 00:01:00\n",
        )
        .expect("parse");
        assert_eq!(sheet.tracks.len(), 1);
        assert!(sheet.unsupported.iter().any(|item| item.contains("MODE1")));
    }

    fn write_wav(path: &std::path::Path, seconds: u32) {
        let frames = 48_000 * seconds;
        let mut bytes = Vec::new();
        let data_bytes = frames * 2;
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&48_000_u32.to_le_bytes());
        bytes.extend_from_slice(&(48_000_u32 * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_bytes.to_le_bytes());
        bytes.resize(bytes.len() + (frames as usize) * 2, 0);
        std::fs::write(path, bytes).expect("wav");
    }

    #[test]
    fn importing_a_sheet_places_one_event_per_track_as_one_step() {
        let root = std::env::temp_dir().join(format!("opentrack-cue-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("dir");
        write_wav(&root.join("album.wav"), 4);
        let cue = root.join("album.cue");
        std::fs::write(&cue, SHEET).expect("cue");

        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.tempo_bpm = 120.0;
        app.project_history.clear();

        app.import_audio_cd_from(&cue);
        let track = &app.data.project.tracks[0];
        assert_eq!(track.name, "album");
        assert_eq!(track.clips.len(), 3);
        assert_eq!(track.clips[0].name, "First");
        assert_eq!(track.clips[0].source_offset_seconds, 0.0);
        assert_eq!(track.clips[1].name, "Second");
        assert!((track.clips[1].source_offset_seconds - 1.0).abs() < 1e-6);
        assert!((track.clips[1].start_beats - 2.0).abs() < 1e-6);
        assert_eq!(track.clips[2].name, "Track 03");
        assert!((track.clips[2].duration_seconds - (4.0 - 2.4)).abs() < 1e-3);
        assert!(app.status_message.contains("Imported 3 CD track(s)"));
        assert!(
            app.data
                .project
                .media_pool
                .contains(&root.join("album.wav"))
        );
        app.project_history.finish_frame(&app.data.project, true);
        assert_eq!(app.project_history.steps()[0].label, "Import Audio CD");

        // A cue sheet naming a missing file changes nothing.
        let missing = root.join("missing.cue");
        std::fs::write(
            &missing,
            "FILE \"gone.wav\" WAVE\n TRACK 01 AUDIO\n INDEX 01 00:00:00\n",
        )
        .expect("cue");
        let before = app.data.project.clone();
        app.import_audio_cd_from(&missing);
        assert!(app.status_message.contains("Audio CD import failed"));
        assert_eq!(app.data.project, before);
        let _ = std::fs::remove_dir_all(root);
    }
}
