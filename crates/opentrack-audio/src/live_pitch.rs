//! Prepared pitch-synchronous playback. Preparation owns allocations; sampling is bounded.
use crate::DecodedWav;
use opentrack_core::pitch_time::PitchTimeEdit;
use std::sync::{Arc, atomic::AtomicBool};

#[cfg(test)]
mod tests {
    use super::*;
    use opentrack_core::pitch_time::{PitchAnalysis, PitchNote, PitchPoint};

    #[test]
    #[ignore = "manual controlled comparison; requires explicit EXAMPLE vocal/analysis paths"]
    fn write_t042_controlled_real_vocal_comparisons() {
        let source_path = std::env::var_os("OPENTRACK_T042_VOCAL_SOURCE")
            .expect("set OPENTRACK_T042_VOCAL_SOURCE to the read-only Vocal 01.wav path");
        let edit_path = std::env::var_os("OPENTRACK_T042_PITCH_EDIT")
            .expect("set OPENTRACK_T042_PITCH_EDIT to an extracted PitchTimeEdit JSON path");
        let output_directory = std::env::var_os("OPENTRACK_T042_OUTPUT")
            .expect("set OPENTRACK_T042_OUTPUT to an ignored comparison-artifact directory");
        let source_path = std::path::PathBuf::from(source_path);
        let edit_path = std::path::PathBuf::from(edit_path);
        let output_directory = std::path::PathBuf::from(output_directory);
        std::fs::create_dir_all(&output_directory).unwrap();

        let mut edit: PitchTimeEdit = serde_json::from_slice(
            &std::fs::read(edit_path).expect("read extracted analysis/edit JSON"),
        )
        .expect("decode saved PitchTimeEdit");
        let identity = crate::pitch_render::pitch_source_identity(&source_path).unwrap();
        edit.source_bytes = identity.0;
        edit.source_modified_ns = identity.1;
        let source = Arc::new(DecodedWav::open(&source_path).unwrap());
        assert_eq!(source.sample_rate, 48_000);
        assert_eq!(source.channels, 1);
        assert!((source.duration_seconds() - 16.832).abs() < 0.001);
        assert_eq!(edit.notes.len(), 22);

        let write_source = |name: &str, data: &[f32]| {
            let path = output_directory.join(name);
            let mut writer = hound::WavWriter::create(
                &path,
                hound::WavSpec {
                    sample_rate: source.sample_rate,
                    channels: source.channels,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )
            .unwrap();
            for sample in data {
                writer.write_sample(*sample).unwrap();
            }
            writer.finalize().unwrap();
        };
        let write_live = |name: &str, edit: &PitchTimeEdit| {
            let prepared = LivePitchSource::new(Arc::clone(&source), edit, 120.0).unwrap();
            let path = output_directory.join(name);
            let mut writer = hound::WavWriter::create(
                &path,
                hound::WavSpec {
                    sample_rate: source.sample_rate,
                    channels: source.channels,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                },
            )
            .unwrap();
            let begin = std::time::Instant::now();
            for frame in 0..prepared.frame_count() {
                let (left, _) = prepared.sample_frame(frame).unwrap();
                writer.write_sample(left).unwrap();
            }
            writer.finalize().unwrap();
            println!("{name}: rendered {:?}", begin.elapsed());
            path
        };
        let measure_note_pitch = |name: &str, path: &std::path::Path, edit: &PitchTimeEdit, id| {
            let note = edit.notes.iter().find(|note| note.id == id).unwrap();
            let output_analysis = crate::pitch_analysis::analyze_pitch_file(
                path,
                0.0,
                source.duration_seconds(),
                &std::sync::atomic::AtomicBool::new(false),
            )
            .unwrap();
            let source_start = edit.analysis.source_start_seconds + note.source_start + 0.03;
            let source_end = edit.analysis.source_start_seconds + note.source_end - 0.03;
            let mut cents = Vec::new();
            for point in edit
                .analysis
                .points
                .iter()
                .filter(|point| (source_start..source_end).contains(&point.source_time_seconds))
            {
                let Some(input_hz) = point.frequency_hz else {
                    continue;
                };
                let i = output_analysis
                    .points
                    .partition_point(|candidate| {
                        candidate.source_time_seconds < point.source_time_seconds
                    })
                    .min(output_analysis.points.len().saturating_sub(1));
                let Some(output_hz) = output_analysis.points.get(i).and_then(|p| p.frequency_hz)
                else {
                    continue;
                };
                let source_midi = 69.0 + 12.0 * (input_hz / 440.0).log2();
                let target_hz = 440.0 * 2.0_f32.powf((note.pitch(source_midi) - 69.0) / 12.0);
                cents.push((
                    1200.0 * (output_hz / target_hz).log2().abs(),
                    point.source_time_seconds,
                    output_hz,
                    target_hz,
                ));
            }
            cents.sort_by(|left, right| left.0.total_cmp(&right.0));
            let median = cents[cents.len() / 2].0;
            let p95 = cents[(cents.len() * 95 / 100).min(cents.len() - 1)].0;
            println!(
                "{name} note {id}: voiced frames {}, median target error {:.2}c, p95 {:.2}c",
                cents.len(),
                median,
                p95
            );
            for (error, time, output_hz, target_hz) in cents
                .iter()
                .rev()
                .take(3)
                .filter(|(error, _, _, _)| *error > 300.0)
            {
                println!(
                    "  outlier: {time:.3}s {output_hz:.1}Hz vs {target_hz:.1}Hz target ({error:.1} cents)"
                );
            }
        };

        let mut unedited = edit.clone();
        for note in &mut unedited.notes {
            note.start = note.source_start;
            note.end = note.source_end;
            note.shift = 0.0;
            note.straighten = 0.0;
        }
        write_source("reference-source.wav", &source.samples);

        let mut single_note = unedited.clone();
        single_note
            .notes
            .iter_mut()
            .find(|note| note.id == 16)
            .expect("the saved 2.18-second vocal note")
            .shift = 3.0;
        let single_path = write_live("single-note-plus-3st.wav", &single_note);
        measure_note_pitch("single-note-plus-3st", &single_path, &single_note, 16);

        let mut semitone_correction = unedited.clone();
        for note in &mut semitone_correction.notes {
            note.shift = note.detected_midi.round() - note.detected_midi;
        }
        write_live("all-notes-nearest-semitone.wav", &semitone_correction);

        let mut touching_stress = unedited;
        let boundary = {
            let left = touching_stress
                .notes
                .iter_mut()
                .find(|note| note.id == 4)
                .expect("left touching vocal note");
            assert!((left.end - 2.877).abs() < 0.002);
            left.shift = 2.0;
            left.end
        };
        let right = touching_stress
            .notes
            .iter_mut()
            .find(|note| note.id == 17)
            .expect("right touching vocal note");
        assert!((right.start - boundary).abs() < 1e-6);
        right.shift = -2.0;
        let stress_path = write_live("touching-notes-stress-plus2-minus2st.wav", &touching_stress);
        measure_note_pitch("touching-stress-left", &stress_path, &touching_stress, 4);
        measure_note_pitch("touching-stress-right", &stress_path, &touching_stress, 17);
    }

    #[test]
    fn live_sampling_changes_pitch_and_timing_without_a_render_file_and_preserves_stereo() {
        let path =
            std::env::temp_dir().join(format!("opentrack-live-note-{}.wav", std::process::id()));
        let mut writer = hound::WavWriter::create(
            &path,
            hound::WavSpec {
                sample_rate: 16000,
                channels: 2,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for i in 0..16000 {
            let t = i as f32 / 16000.0;
            let sample = if (0.2..0.6).contains(&t) {
                0.4 * (std::f32::consts::TAU * 220.0 * t).sin()
            } else {
                0.0
            };
            writer.write_sample(sample).unwrap();
            writer.write_sample(sample * 0.37).unwrap();
        }
        writer.finalize().unwrap();
        let source = Arc::new(DecodedWav::open(&path).unwrap());
        std::fs::remove_file(path).unwrap();
        let edit = PitchTimeEdit {
            detector_version: 2,
            source_bytes: 0,
            source_modified_ns: 0,
            enabled: true,
            rendered_source: None,
            analysis: PitchAnalysis {
                sample_rate_hz: 16000,
                source_start_seconds: 0.0,
                source_duration_seconds: 1.0,
                points: (0..100)
                    .map(|i| PitchPoint {
                        source_time_seconds: i as f64 * 0.01,
                        frequency_hz: Some(220.0),
                        confidence: 1.0,
                    })
                    .collect(),
            },
            notes: vec![PitchNote {
                id: 1,
                source_start: 0.2,
                source_end: 0.6,
                start: 0.25,
                end: 0.9,
                detected_midi: 57.0,
                shift: 7.0,
                straighten: 0.0,
                pitch_start_bend: 0.0,
                pitch_anchor: 0.5,
                pitch_anchor_bend: 0.0,
                pitch_end_bend: 0.0,
            }],
        };
        let prepared = LivePitchSource::new(source, &edit, 120.0).unwrap();
        let begin = std::time::Instant::now();
        let samples: Vec<_> = (0..16000)
            .map(|i| prepared.sample_frame(i).unwrap())
            .collect();
        println!("Live sampling 1 second stereo: {:?}", begin.elapsed());
        assert!(samples.iter().all(|(l, r)| (r - 0.37 * l).abs() < 1e-5));
        assert!(samples[13000..13800].iter().any(|s| s.0.abs() > 0.1));
        let mono: Vec<_> = samples.iter().map(|s| s.0).collect();
        let pitches = crate::pitch_analysis::detect_pitch(
            &mono,
            0.0,
            &std::sync::atomic::AtomicBool::new(false),
        );
        let mut errors: Vec<_> = pitches
            .iter()
            .filter(|p| (0.4..0.75).contains(&p.source_time_seconds))
            .map(|p| 1200.0 * (p.frequency_hz.unwrap() / 329.62756).log2().abs())
            .collect();
        errors.sort_by(f32::total_cmp);
        assert!(errors[errors.len() / 2] < 25.0);
    }
}
#[derive(Debug)]
pub struct LivePitchSource {
    source: Arc<DecodedWav>,
    synthesis: crate::pitch_synthesis::PitchSynthesis,
}

impl LivePitchSource {
    pub fn new(
        source: Arc<DecodedWav>,
        edit: &PitchTimeEdit,
        tempo_bpm: f64,
    ) -> Result<Self, String> {
        let cancel = AtomicBool::new(false);
        let synthesis = crate::pitch_synthesis::PitchSynthesis::new(
            source.samples.as_ref(),
            source.sample_rate,
            usize::from(source.channels),
            edit,
            tempo_bpm,
            &cancel,
        )?;
        Ok(Self { source, synthesis })
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.source.sample_rate
    }

    pub(crate) fn frame_count(&self) -> u64 {
        (self.source.samples.len() / usize::from(self.source.channels)) as u64
    }

    pub(crate) fn sample_frame(&self, frame: u64) -> Option<(f32, f32)> {
        if frame >= self.frame_count() {
            return None;
        }
        self.synthesis
            .sample_frame(self.source.samples.as_ref(), frame)
    }
}
