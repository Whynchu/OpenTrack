//! Media > Reconstruct: re-read the project's audio sources from disk.
//!
//! Waveforms and playback read their samples from the decoded/streamed sources
//! the app holds in memory, so rebuilding those sources from their files is the
//! honest meaning of "reconstruct" here: after a file is replaced or repaired
//! outside OpenTrack, this reloads it and rebuilds the playback plan. Nothing is
//! derived or cached separately, so there is no second cache to regenerate.

use super::*;
use std::collections::BTreeSet;

impl OpenTrackApp {
    pub(super) fn reconstruct_audio_sources(&mut self) {
        if !self.project_open {
            self.status_message = "Open a project before reconstructing its audio".to_owned();
            return;
        }
        let mut paths = BTreeSet::new();
        paths.extend(self.data.project.media_pool.iter().cloned());
        for track in self
            .data
            .project
            .tracks
            .iter()
            .chain(std::iter::once(&self.data.project.output_channel.strip))
        {
            for clip in &track.clips {
                paths.insert(clip.source_path.clone());
            }
        }
        if paths.is_empty() {
            self.status_message = "The project has no audio sources to reconstruct".to_owned();
            return;
        }
        let mut rebuilt = 0usize;
        let mut missing = Vec::new();
        let mut sources = std::collections::HashMap::new();
        for path in paths {
            match AudioSource::open(&path) {
                Ok(source) => {
                    sources.insert(path, source);
                    rebuilt += 1;
                }
                Err(_) => missing.push(path),
            }
        }
        self.data.audio_sources = sources;
        self.rebuild_audio_plan();
        if missing.is_empty() {
            self.status_message = format!(
                "Reconstructed {rebuilt} audio source(s) from disk and rebuilt the playback plan"
            );
        } else {
            let names = missing
                .iter()
                .take(3)
                .map(|path| {
                    path.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string())
                })
                .collect::<Vec<_>>()
                .join(", ");
            self.status_message = format!(
                "Reconstructed {rebuilt} audio source(s); {} unavailable ({names}{})",
                missing.len(),
                if missing.len() > 3 { ", …" } else { "" }
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(path: &Path, frames: u32) {
        let rate = 48_000u32;
        let mut bytes = Vec::new();
        let data_bytes = frames * 2;
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * 2).to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_bytes.to_le_bytes());
        bytes.resize(bytes.len() + (frames as usize) * 2, 0);
        std::fs::write(path, bytes).expect("wav");
    }

    fn app_with_clip(path: &Path) -> OpenTrackApp {
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut app = OpenTrackApp::new_internal(&creation, false);
        app.project_open = true;
        app.data.project.add_track(TrackKind::Audio);
        app.data
            .project
            .add_audio_clip(0, path.to_path_buf(), 0.0, 1.0)
            .expect("clip");
        app.data.audio_sources.clear();
        app
    }

    #[test]
    fn reconstruct_reloads_sources_and_reports_missing_files() {
        let root =
            std::env::temp_dir().join(format!("opentrack-reconstruct-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let source = root.join("take.wav");
        write_wav(&source, 24_000);
        let mut app = app_with_clip(&source);
        assert!(app.data.audio_sources.is_empty());

        app.reconstruct_audio_sources();
        assert_eq!(app.data.audio_sources.len(), 1);
        assert!(
            app.status_message
                .contains("Reconstructed 1 audio source(s)")
        );

        // A file removed outside OpenTrack is reported, not invented.
        std::fs::remove_file(&source).expect("remove");
        app.reconstruct_audio_sources();
        assert!(app.data.audio_sources.is_empty());
        assert!(
            app.status_message.contains("1 unavailable"),
            "{}",
            app.status_message
        );
        assert!(app.status_message.contains("take.wav"));

        // Without a project the command reports.
        let ctx = egui::Context::default();
        let creation = eframe::CreationContext::_new_kittest(ctx);
        let mut closed = OpenTrackApp::new_internal(&creation, false);
        closed.reconstruct_audio_sources();
        assert!(closed.status_message.contains("Open a project"));
        let _ = std::fs::remove_dir_all(root);
    }
}
