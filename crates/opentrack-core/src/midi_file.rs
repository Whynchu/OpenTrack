//! Standard MIDI File interchange for OpenTrack's supported note/CC model.

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
};

use crate::project::{Project, TrackKind};

const PPQ: u16 = 480;
const MAX_FILE_BYTES: usize = 128 * 1024 * 1024;
const MAX_TRACKS: usize = 1025;
const MAX_EVENTS: usize = 2_000_000;
const MAX_VLQ: u64 = 0x0fff_ffff;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MidiImportReport {
    pub tracks_added: usize,
    pub notes_imported: usize,
    pub controllers_imported: usize,
    pub ignored_events: usize,
    pub tempo_events_ignored: usize,
    pub meta_events_ignored: usize,
    pub sysex_events_ignored: usize,
    pub unsupported_channel_events: usize,
    pub unmatched_note_events: usize,
    pub duplicate_controller_events: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MidiExportReport {
    pub tracks_written: usize,
    pub notes_written: usize,
    pub controllers_written: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MidiFileError {
    FileTooLarge,
    Truncated,
    InvalidHeader,
    UnsupportedFormat(u16),
    UnsupportedDivision,
    InvalidChunk,
    InvalidEvent,
    InvalidVariableLength,
    TempoOutOfRange,
    ProjectLimit(&'static str),
    InvalidProject(&'static str),
}

impl fmt::Display for MidiFileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileTooLarge => formatter.write_str("MIDI file exceeds the 128 MiB import limit"),
            Self::Truncated => formatter.write_str("MIDI file ended before an event was complete"),
            Self::InvalidHeader => {
                formatter.write_str("file does not have a valid Standard MIDI File header")
            }
            Self::UnsupportedFormat(format) => write!(
                formatter,
                "Standard MIDI File format {format} is not supported (use format 0 or 1)"
            ),
            Self::UnsupportedDivision => formatter
                .write_str("SMPTE MIDI timing is not supported; save the file with PPQ timing"),
            Self::InvalidChunk => {
                formatter.write_str("MIDI file contains an invalid or incomplete track chunk")
            }
            Self::InvalidEvent => formatter.write_str("MIDI track contains an invalid event"),
            Self::InvalidVariableLength => {
                formatter.write_str("MIDI event has an invalid variable-length quantity")
            }
            Self::TempoOutOfRange => formatter.write_str(
                "project tempo cannot be represented by the Standard MIDI File tempo field",
            ),
            Self::ProjectLimit(message) => {
                write!(formatter, "MIDI import exceeds a project limit: {message}")
            }
            Self::InvalidProject(message) => write!(
                formatter,
                "MIDI import would create an invalid project: {message}"
            ),
        }
    }
}

impl std::error::Error for MidiFileError {}

#[derive(Clone, Copy, Debug)]
enum EventKind {
    NoteOn {
        channel: u8,
        pitch: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        pitch: u8,
    },
    Controller {
        channel: u8,
        controller: u8,
        value: u8,
    },
    Other,
    Tempo,
}

#[derive(Clone, Copy, Debug)]
struct TimedEvent {
    tick: u64,
    kind: EventKind,
}

#[derive(Default)]
struct ParsedTrack {
    name: Option<String>,
    events: Vec<TimedEvent>,
    end_tick: u64,
}

/// Imports note and CC events into new MIDI tracks at project beat zero.
/// Parsing and validation complete on a clone before the live project changes.
pub fn import_standard_midi(
    project: &mut Project,
    bytes: &[u8],
) -> Result<MidiImportReport, MidiFileError> {
    let (ppq, tracks, mut report) = parse_file(bytes)?;
    let mut candidate = project.clone();
    for source_track in tracks {
        let source_name = source_track.name.clone();
        let (notes, controllers, end_tick, ignored, unsupported_channel, unmatched_notes) =
            normalize_track(source_track);
        report.ignored_events += ignored;
        report.unsupported_channel_events += unsupported_channel;
        report.unmatched_note_events += unmatched_notes;
        if notes.is_empty() && controllers.is_empty() {
            continue;
        }
        if candidate.tracks.len() >= 1024 {
            return Err(MidiFileError::ProjectLimit(
                "project cannot contain more than 1024 tracks",
            ));
        }
        if candidate.next_track_id >= crate::project::INPUT_BUS_ID
            || candidate.next_track_number == u32::MAX
        {
            return Err(MidiFileError::ProjectLimit(
                "track identity space is exhausted",
            ));
        }
        candidate.add_track(TrackKind::Midi);
        let track_index = candidate.tracks.len() - 1;
        if let Some(name) = source_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let name: String = name
                .chars()
                .filter(|character| !character.is_control())
                .take(128)
                .collect();
            if !name.trim().is_empty() {
                candidate.tracks[track_index].name = name;
            }
        }
        let part_end = ticks_to_beats(end_tick.max(1), ppq).max(1.0);
        let part_id = candidate.add_midi_part(track_index, 0.0, part_end).ok_or(
            MidiFileError::ProjectLimit("MIDI part identity space is exhausted"),
        )?;
        for note in notes {
            if candidate
                .add_midi_note_on_channel(
                    track_index,
                    part_id,
                    ticks_to_beats(note.start_tick, ppq),
                    ticks_to_beats(note.end_tick.saturating_sub(note.start_tick), ppq),
                    note.pitch,
                    note.channel,
                    note.velocity,
                )
                .is_none()
            {
                return Err(MidiFileError::ProjectLimit(
                    "MIDI note identity space is exhausted",
                ));
            }
            report.notes_imported += 1;
        }
        for event in controllers {
            let duplicate = candidate.tracks[track_index].midi_parts.iter().any(|part| {
                part.id == part_id
                    && part.controller_events.iter().any(|existing| {
                        existing.position_beats == ticks_to_beats(event.tick, ppq)
                            && existing.channel == event.channel
                            && existing.controller == event.controller
                    })
            });
            if candidate
                .add_midi_controller_event_on_channel(
                    track_index,
                    part_id,
                    ticks_to_beats(event.tick, ppq),
                    event.controller,
                    event.channel,
                    event.value,
                )
                .is_none()
            {
                return Err(MidiFileError::ProjectLimit(
                    "MIDI controller identity space is exhausted",
                ));
            }
            if duplicate {
                report.ignored_events += 1;
                report.duplicate_controller_events += 1;
            } else {
                report.controllers_imported += 1;
            }
        }
        report.tracks_added += 1;
    }
    candidate
        .validate()
        .map_err(MidiFileError::InvalidProject)?;
    *project = candidate;
    Ok(report)
}

#[derive(Clone, Copy)]
struct ImportedNote {
    start_tick: u64,
    end_tick: u64,
    pitch: u8,
    channel: u8,
    velocity: u8,
}

#[derive(Clone, Copy)]
struct ImportedController {
    tick: u64,
    channel: u8,
    controller: u8,
    value: u8,
}

fn normalize_track(
    track: ParsedTrack,
) -> (
    Vec<ImportedNote>,
    Vec<ImportedController>,
    u64,
    usize,
    usize,
    usize,
) {
    let mut active: BTreeMap<(u8, u8), VecDeque<(u64, u8)>> = BTreeMap::new();
    let mut notes = Vec::new();
    let mut controllers = Vec::new();
    let mut ignored = 0;
    let mut unsupported_channel = 0;
    let mut unmatched_notes = 0;
    let end_tick = track
        .end_tick
        .max(track.events.last().map_or(0, |event| event.tick));
    for event in track.events {
        match event.kind {
            EventKind::NoteOn {
                channel,
                pitch,
                velocity,
            } if velocity > 0 => {
                active
                    .entry((channel, pitch))
                    .or_default()
                    .push_back((event.tick, velocity));
            }
            EventKind::NoteOn { channel, pitch, .. } | EventKind::NoteOff { channel, pitch } => {
                if let Some((start_tick, velocity)) = active
                    .get_mut(&(channel, pitch))
                    .and_then(VecDeque::pop_front)
                {
                    if event.tick > start_tick {
                        notes.push(ImportedNote {
                            start_tick,
                            end_tick: event.tick,
                            pitch,
                            channel,
                            velocity,
                        });
                    } else {
                        ignored += 1;
                        unmatched_notes += 1;
                    }
                } else {
                    ignored += 1;
                    unmatched_notes += 1;
                }
            }
            EventKind::Controller {
                channel,
                controller,
                value,
            } => {
                controllers.push(ImportedController {
                    tick: event.tick,
                    channel,
                    controller,
                    value,
                });
            }
            EventKind::Other => {
                ignored += 1;
                unsupported_channel += 1;
            }
            EventKind::Tempo => {}
        }
    }
    for ((channel, pitch), mut starts) in active {
        while let Some((start_tick, velocity)) = starts.pop_front() {
            if end_tick > start_tick {
                notes.push(ImportedNote {
                    start_tick,
                    end_tick,
                    pitch,
                    channel,
                    velocity,
                });
            } else {
                ignored += 1;
                unmatched_notes += 1;
            }
        }
    }
    notes.sort_by_key(|note| (note.start_tick, note.channel, note.pitch));
    controllers.sort_by_key(|event| (event.tick, event.channel, event.controller));
    (
        notes,
        controllers,
        end_tick,
        ignored,
        unsupported_channel,
        unmatched_notes,
    )
}

fn ticks_to_beats(tick: u64, ppq: u16) -> f64 {
    tick as f64 / f64::from(ppq)
}

fn parse_file(bytes: &[u8]) -> Result<(u16, Vec<ParsedTrack>, MidiImportReport), MidiFileError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(MidiFileError::FileTooLarge);
    }
    if bytes.get(..4) != Some(b"MThd") || bytes.len() < 14 {
        return Err(MidiFileError::InvalidHeader);
    }
    let header_len = read_u32(bytes, 4)? as usize;
    if header_len < 6
        || 8_usize
            .checked_add(header_len)
            .is_none_or(|end| end > bytes.len())
    {
        return Err(MidiFileError::InvalidHeader);
    }
    let format = read_u16(bytes, 8)?;
    if !matches!(format, 0 | 1) {
        return Err(MidiFileError::UnsupportedFormat(format));
    }
    let track_count = read_u16(bytes, 10)? as usize;
    if track_count == 0 || (format == 0 && track_count != 1) {
        return Err(MidiFileError::InvalidHeader);
    }
    if track_count > MAX_TRACKS {
        return Err(MidiFileError::ProjectLimit(
            "MIDI file contains more than 1025 source tracks",
        ));
    }
    let division = read_u16(bytes, 12)?;
    if division & 0x8000 != 0 || division == 0 {
        return Err(MidiFileError::UnsupportedDivision);
    }
    let mut cursor = 8 + header_len;
    let mut tracks = Vec::with_capacity(track_count);
    let mut report = MidiImportReport::default();
    let mut event_count = 0;
    for _ in 0..track_count {
        if bytes.get(cursor..cursor + 4) != Some(b"MTrk") {
            return Err(MidiFileError::InvalidChunk);
        }
        let length = read_u32(bytes, cursor + 4)? as usize;
        cursor = cursor.checked_add(8).ok_or(MidiFileError::InvalidChunk)?;
        let end = cursor
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .ok_or(MidiFileError::InvalidChunk)?;
        tracks.push(parse_track(
            &bytes[cursor..end],
            &mut report,
            &mut event_count,
        )?);
        cursor = end;
    }
    if cursor != bytes.len() {
        return Err(MidiFileError::InvalidChunk);
    }
    Ok((division, tracks, report))
}

fn parse_track(
    bytes: &[u8],
    report: &mut MidiImportReport,
    event_count: &mut usize,
) -> Result<ParsedTrack, MidiFileError> {
    let mut result = ParsedTrack::default();
    let mut cursor = 0;
    let mut absolute_tick = 0_u64;
    let mut running_status = None;
    while cursor < bytes.len() {
        let delta = read_vlq(bytes, &mut cursor)? as u64;
        absolute_tick = absolute_tick
            .checked_add(delta)
            .ok_or(MidiFileError::InvalidEvent)?;
        let first = *bytes.get(cursor).ok_or(MidiFileError::Truncated)?;
        let status = if first & 0x80 != 0 {
            cursor += 1;
            if (0x80..=0xef).contains(&first) {
                running_status = Some(first);
            } else {
                running_status = None;
            }
            first
        } else {
            cursor += 1;
            running_status.ok_or(MidiFileError::InvalidEvent)?
        };
        match status {
            0x80..=0xef => {
                let message = status & 0xf0;
                let channel = status & 0x0f;
                let data_len = if matches!(message, 0xc0 | 0xd0) { 1 } else { 2 };
                let first_data = if first & 0x80 == 0 { Some(first) } else { None };
                let data1 = match first_data {
                    Some(value) => value,
                    None => take_data(bytes, &mut cursor)?,
                };
                let data2 = if data_len == 2 {
                    Some(take_data(bytes, &mut cursor)?)
                } else {
                    None
                };
                let kind = match message {
                    0x80 => EventKind::NoteOff {
                        channel,
                        pitch: data1,
                    },
                    0x90 if data2 == Some(0) => EventKind::NoteOff {
                        channel,
                        pitch: data1,
                    },
                    0x90 => EventKind::NoteOn {
                        channel,
                        pitch: data1,
                        velocity: data2.unwrap_or(0),
                    },
                    0xb0 => EventKind::Controller {
                        channel,
                        controller: data1,
                        value: data2.unwrap_or(0),
                    },
                    _ => EventKind::Other,
                };
                push_timed_event(
                    &mut result.events,
                    TimedEvent {
                        tick: absolute_tick,
                        kind,
                    },
                    event_count,
                )?;
            }
            0xff => {
                let meta_type = *bytes.get(cursor).ok_or(MidiFileError::Truncated)?;
                cursor += 1;
                let length = read_vlq(bytes, &mut cursor)? as usize;
                let payload = take_slice(bytes, &mut cursor, length)?;
                match meta_type {
                    0x2f => {
                        if !payload.is_empty() {
                            return Err(MidiFileError::InvalidEvent);
                        }
                        result.end_tick = absolute_tick;
                        break;
                    }
                    0x03 => {
                        result.name = Some(
                            String::from_utf8_lossy(payload)
                                .trim_matches('\0')
                                .to_owned(),
                        )
                    }
                    0x51 => {
                        if payload.len() != 3 || payload.iter().all(|byte| *byte == 0) {
                            return Err(MidiFileError::InvalidEvent);
                        }
                        report.tempo_events_ignored += 1;
                        report.ignored_events += 1;
                        push_timed_event(
                            &mut result.events,
                            TimedEvent {
                                tick: absolute_tick,
                                kind: EventKind::Tempo,
                            },
                            event_count,
                        )?;
                    }
                    _ => {
                        report.ignored_events += 1;
                        report.meta_events_ignored += 1;
                    }
                }
            }
            0xf0 | 0xf7 => {
                let length = read_vlq(bytes, &mut cursor)? as usize;
                take_slice(bytes, &mut cursor, length)?;
                report.ignored_events += 1;
                report.sysex_events_ignored += 1;
            }
            _ => return Err(MidiFileError::InvalidEvent),
        }
    }
    Ok(result)
}

fn push_timed_event(
    events: &mut Vec<TimedEvent>,
    event: TimedEvent,
    event_count: &mut usize,
) -> Result<(), MidiFileError> {
    *event_count += 1;
    if *event_count > MAX_EVENTS {
        return Err(MidiFileError::ProjectLimit(
            "MIDI file contains more than 2,000,000 channel/tempo events",
        ));
    }
    events.push(event);
    Ok(())
}

fn take_data(bytes: &[u8], cursor: &mut usize) -> Result<u8, MidiFileError> {
    let value = *bytes.get(*cursor).ok_or(MidiFileError::Truncated)?;
    if value & 0x80 != 0 {
        return Err(MidiFileError::InvalidEvent);
    }
    *cursor += 1;
    Ok(value)
}

fn take_slice<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], MidiFileError> {
    let end = cursor.checked_add(length).ok_or(MidiFileError::Truncated)?;
    let payload = bytes.get(*cursor..end).ok_or(MidiFileError::Truncated)?;
    *cursor = end;
    Ok(payload)
}

fn read_vlq(bytes: &[u8], cursor: &mut usize) -> Result<u32, MidiFileError> {
    let mut value = 0_u32;
    for index in 0..4 {
        let byte = *bytes.get(*cursor).ok_or(MidiFileError::Truncated)?;
        *cursor += 1;
        value = value
            .checked_shl(7)
            .ok_or(MidiFileError::InvalidVariableLength)?
            | u32::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        if index == 3 {
            return Err(MidiFileError::InvalidVariableLength);
        }
    }
    Err(MidiFileError::InvalidVariableLength)
}

fn read_u16(bytes: &[u8], cursor: usize) -> Result<u16, MidiFileError> {
    let pair: [u8; 2] = bytes
        .get(cursor..cursor + 2)
        .ok_or(MidiFileError::Truncated)?
        .try_into()
        .map_err(|_| MidiFileError::Truncated)?;
    Ok(u16::from_be_bytes(pair))
}

fn read_u32(bytes: &[u8], cursor: usize) -> Result<u32, MidiFileError> {
    let quad: [u8; 4] = bytes
        .get(cursor..cursor + 4)
        .ok_or(MidiFileError::Truncated)?
        .try_into()
        .map_err(|_| MidiFileError::Truncated)?;
    Ok(u32::from_be_bytes(quad))
}

#[derive(Clone)]
struct OutputEvent {
    tick: u64,
    order: u8,
    data: Vec<u8>,
}

/// What part of a project a MIDI export should contain.
///
/// The default (everything empty) is the whole project, which is what the
/// existing "MIDI File..." command exports. A selection narrows it to specific
/// tracks or parts, optionally limited to a beat range, so a user can hand a
/// collaborator one part or one loop without cutting the project first.
#[derive(Clone, Debug, Default)]
pub struct MidiExportScope {
    /// Track ids to include. Empty means every MIDI track.
    pub track_ids: Vec<u64>,
    /// Part ids to include. Empty means every part on the chosen tracks.
    pub part_ids: Vec<u64>,
    /// Start of the exported range, or `None` for the part's own start.
    pub start_beats: Option<f64>,
    /// End of the exported range, or `None` for the part's own end.
    pub end_beats: Option<f64>,
}

impl MidiExportScope {
    pub fn is_whole_project(&self) -> bool {
        self.track_ids.is_empty()
            && self.part_ids.is_empty()
            && self.start_beats.is_none()
            && self.end_beats.is_none()
    }

    /// The chosen range when both ends are present and form a real interval.
    fn range(&self) -> Option<(f64, f64)> {
        match (self.start_beats, self.end_beats) {
            (Some(start), Some(end))
                if start.is_finite() && end.is_finite() && end > start && start >= 0.0 =>
            {
                Some((start, end))
            }
            _ => None,
        }
    }
}

/// Encodes supported project notes and CC events as a format-1, 480-PPQ file.
pub fn export_standard_midi(
    project: &Project,
) -> Result<(Vec<u8>, MidiExportReport), MidiFileError> {
    export_standard_midi_scope(project, &MidiExportScope::default())
}

/// Encodes the chosen tracks, parts and range as a format-1, 480-PPQ file.
///
/// A note that crosses the range boundary is clipped to it rather than dropped,
/// so a loop export still ends on the note that sounds at the boundary. A note
/// entirely outside the range is left out, and controller events follow the same
/// rule.
pub fn export_standard_midi_scope(
    project: &Project,
    scope: &MidiExportScope,
) -> Result<(Vec<u8>, MidiExportReport), MidiFileError> {
    project.validate().map_err(MidiFileError::InvalidProject)?;
    let tempo_micros = (60_000_000.0 / project.tempo_bpm).round();
    if !(1.0..=16_777_215.0).contains(&tempo_micros) {
        return Err(MidiFileError::TempoOutOfRange);
    }
    let mut output_tracks = Vec::new();
    let mut conductor = vec![
        OutputEvent {
            tick: 0,
            order: 0,
            data: meta_event(0x03, b"OpenTrack Tempo"),
        },
        OutputEvent {
            tick: 0,
            order: 0,
            data: meta_event(0x51, &(tempo_micros as u32).to_be_bytes()[1..]),
        },
    ];
    let denominator_power = project.beat_unit.trailing_zeros() as u8;
    conductor.push(OutputEvent {
        tick: 0,
        order: 0,
        data: meta_event(0x58, &[project.beats_per_bar, denominator_power, 24, 8]),
    });
    output_tracks.push(encode_track(conductor)?);
    let mut report = MidiExportReport::default();
    let range = scope.range();
    for track in &project.tracks {
        if !matches!(track.kind, TrackKind::Midi | TrackKind::Instrument)
            || track.midi_parts.is_empty()
        {
            continue;
        }
        if !scope.track_ids.is_empty() && !scope.track_ids.contains(&track.id) {
            continue;
        }
        let mut events = Vec::new();
        events.push(OutputEvent {
            tick: 0,
            order: 0,
            data: meta_event(0x03, track.name.as_bytes()),
        });
        let notes_before = report.notes_written;
        let controllers_before = report.controllers_written;
        for part in &track.midi_parts {
            if !scope.part_ids.is_empty() && !scope.part_ids.contains(&part.id) {
                continue;
            }
            for note in &part.notes {
                let note_start = part.start_beats + note.start_beats;
                let note_end = note_start + note.duration_beats;
                let (start_beats, end_beats) = match range {
                    Some((start, end)) => {
                        if note_end <= start || note_start >= end {
                            continue;
                        }
                        (note_start.max(start), note_end.min(end))
                    }
                    None => (note_start, note_end),
                };
                let start = beats_to_ticks(start_beats)?;
                let end = beats_to_ticks(end_beats)?;
                if end <= start {
                    continue;
                }
                events.push(OutputEvent {
                    tick: start,
                    order: 2,
                    data: vec![
                        0x90 | (note.channel & 0x0f),
                        note.pitch.min(127),
                        note.velocity.clamp(1, 127),
                    ],
                });
                events.push(OutputEvent {
                    tick: end,
                    order: 1,
                    data: vec![0x80 | (note.channel & 0x0f), note.pitch.min(127), 0],
                });
                report.notes_written += 1;
            }
            for event in &part.controller_events {
                let position = part.start_beats + event.position_beats;
                if let Some((start, end)) = range
                    && (position < start || position >= end)
                {
                    continue;
                }
                events.push(OutputEvent {
                    tick: beats_to_ticks(position)?,
                    order: 1,
                    data: vec![
                        0xb0 | (event.channel & 0x0f),
                        event.controller.min(127),
                        event.value.min(127),
                    ],
                });
                report.controllers_written += 1;
            }
        }
        // A selection export leaves out a track that contributes nothing to it,
        // so a loop export of one part does not carry every other track's name.
        if scope.track_ids.is_empty()
            && scope.part_ids.is_empty()
            && range.is_none()
            && report.notes_written == notes_before
            && report.controllers_written == controllers_before
        {
            // The whole-project export keeps its historic shape, including a
            // track whose parts are empty.
        } else if report.notes_written == notes_before
            && report.controllers_written == controllers_before
        {
            continue;
        }
        events.sort_by_key(|event| (event.tick, event.order));
        output_tracks.push(encode_track(events)?);
    }
    report.tracks_written = output_tracks.len();
    if output_tracks.len() == 1 {
        // Format 0 requires exactly one track; the conductor is sufficient for an empty project.
    }
    let format = if output_tracks.len() == 1 {
        0_u16
    } else {
        1_u16
    };
    let track_count = u16::try_from(output_tracks.len())
        .map_err(|_| MidiFileError::ProjectLimit("too many MIDI tracks"))?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6_u32.to_be_bytes());
    bytes.extend_from_slice(&format.to_be_bytes());
    bytes.extend_from_slice(&track_count.to_be_bytes());
    bytes.extend_from_slice(&PPQ.to_be_bytes());
    for track in output_tracks {
        bytes.extend_from_slice(&track);
    }
    if bytes.len() > MAX_FILE_BYTES {
        return Err(MidiFileError::FileTooLarge);
    }
    Ok((bytes, report))
}

fn beats_to_ticks(beats: f64) -> Result<u64, MidiFileError> {
    let ticks = beats * f64::from(PPQ);
    if !ticks.is_finite() || ticks < 0.0 || ticks >= u64::MAX as f64 {
        return Err(MidiFileError::InvalidProject(
            "MIDI beat position is outside the supported range",
        ));
    }
    Ok(ticks.round() as u64)
}

fn meta_event(meta_type: u8, payload: &[u8]) -> Vec<u8> {
    let mut event = vec![0xff, meta_type];
    put_vlq(payload.len() as u64, &mut event);
    event.extend_from_slice(payload);
    event
}

fn encode_track(mut events: Vec<OutputEvent>) -> Result<Vec<u8>, MidiFileError> {
    events.sort_by_key(|event| (event.tick, event.order));
    let mut track_data = Vec::new();
    let mut previous_tick = 0_u64;
    for event in events {
        let delta = event.tick.saturating_sub(previous_tick);
        put_delta(delta, &mut track_data);
        track_data.extend_from_slice(&event.data);
        previous_tick = event.tick;
    }
    put_vlq(0, &mut track_data);
    track_data.extend_from_slice(&[0xff, 0x2f, 0x00]);
    let length = u32::try_from(track_data.len()).map_err(|_| MidiFileError::FileTooLarge)?;
    let mut chunk = Vec::with_capacity(track_data.len() + 8);
    chunk.extend_from_slice(b"MTrk");
    chunk.extend_from_slice(&length.to_be_bytes());
    chunk.extend_from_slice(&track_data);
    Ok(chunk)
}

fn put_vlq(value: u64, output: &mut Vec<u8>) {
    let mut buffer = [0_u8; 4];
    let mut index = 3;
    buffer[index] = (value & 0x7f) as u8;
    let mut value = value >> 7;
    while value > 0 && index > 0 {
        index -= 1;
        buffer[index] = ((value & 0x7f) as u8) | 0x80;
        value >>= 7;
    }
    output.extend_from_slice(&buffer[index..]);
}

fn put_delta(mut delta: u64, output: &mut Vec<u8>) {
    while delta > MAX_VLQ {
        put_vlq(MAX_VLQ, output);
        // A zero-length sequencer-specific meta event safely consumes the
        // largest legal delta while leaving the musical event unchanged.
        output.extend_from_slice(&[0xff, 0x7f, 0x00]);
        delta -= MAX_VLQ;
    }
    put_vlq(delta, output);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_zero_track(track: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6_u32.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&480_u16.to_be_bytes());
        bytes.extend_from_slice(b"MTrk");
        bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
        bytes.extend_from_slice(track);
        bytes
    }

    fn format_one_tracks(tracks: &[&[u8]]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"MThd");
        bytes.extend_from_slice(&6_u32.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
        bytes.extend_from_slice(&480_u16.to_be_bytes());
        for track in tracks {
            bytes.extend_from_slice(b"MTrk");
            bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
            bytes.extend_from_slice(track);
        }
        bytes
    }

    // This test decoder intentionally does not call the production SMF parser.
    // It reads the export subset directly from the file and returns absolute
    // tick positions with the original event payloads.
    fn independently_decode_export(bytes: &[u8]) -> (u16, u16, Vec<Vec<(u64, Vec<u8>)>>) {
        fn u16_at(bytes: &[u8], offset: usize) -> u16 {
            u16::from_be_bytes([bytes[offset], bytes[offset + 1]])
        }

        fn u32_at(bytes: &[u8], offset: usize) -> usize {
            u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize
        }

        fn vlq(bytes: &[u8], cursor: &mut usize) -> u32 {
            let mut value = 0;
            loop {
                let byte = bytes[*cursor];
                *cursor += 1;
                value = (value << 7) | u32::from(byte & 0x7f);
                if byte & 0x80 == 0 {
                    return value;
                }
            }
        }

        assert_eq!(&bytes[..4], b"MThd");
        assert_eq!(u32_at(bytes, 4), 6);
        let format = u16_at(bytes, 8);
        let track_count = u16_at(bytes, 10);
        let division = u16_at(bytes, 12);
        let mut cursor = 14;
        let mut decoded_tracks = Vec::new();
        for _ in 0..track_count {
            assert_eq!(&bytes[cursor..cursor + 4], b"MTrk");
            let length = u32_at(bytes, cursor + 4);
            let start = cursor + 8;
            let end = start + length;
            let track = &bytes[start..end];
            cursor = end;
            let mut position = 0;
            let mut absolute_tick = 0_u64;
            let mut events = Vec::new();
            while position < track.len() {
                absolute_tick += u64::from(vlq(track, &mut position));
                let status = track[position];
                position += 1;
                let payload = if status == 0xff {
                    let meta_type = track[position];
                    position += 1;
                    let length = vlq(track, &mut position) as usize;
                    let data = track[position..position + length].to_vec();
                    position += length;
                    let mut event = vec![0xff, meta_type];
                    event.extend(data);
                    event
                } else {
                    let length = if matches!(status & 0xf0, 0xc0 | 0xd0) {
                        1
                    } else {
                        2
                    };
                    let mut event = vec![status];
                    event.extend_from_slice(&track[position..position + length]);
                    position += length;
                    event
                };
                events.push((absolute_tick, payload));
            }
            decoded_tracks.push(events);
        }
        assert_eq!(cursor, bytes.len());
        (format, division, decoded_tracks)
    }

    fn project_with_midi() -> Project {
        let mut project = Project::default();
        project.name = "SMF Fixture".to_owned();
        project.tempo_bpm = 123.0;
        project.beats_per_bar = 7;
        project.beat_unit = 8;
        project.add_track(TrackKind::Midi);
        project.tracks[0].name = "Bass".to_owned();
        let part = project.add_midi_part(0, 4.0, 4.0).unwrap();
        project
            .add_midi_note_on_channel(0, part, 0.5, 1.25, 60, 2, 96)
            .unwrap();
        project
            .add_midi_controller_event_on_channel(0, part, 2.0, 11, 3, 81)
            .unwrap();
        project
    }

    #[test]
    fn imports_format_zero_running_status_notes_and_controllers() {
        let track = vec![
            0x00, 0xff, 0x03, 0x04, b'B', b'a', b's', b's', 0x00, 0x92, 0x3c, 0x5a, 0x83, 0x60,
            0x3c, 0x00, // one beat later, running status note-off
            0x00, 0xb2, 0x0b, 0x40, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00, 0xff, 0x2f,
            0x00,
        ];
        // Test fixtures are deliberately small and self-contained.
        let bytes = format_zero_track(&track);
        let mut project = Project::default();
        let report = import_standard_midi(&mut project, &bytes).unwrap();
        assert_eq!(report.tracks_added, 1);
        assert_eq!(report.notes_imported, 1);
        assert_eq!(report.controllers_imported, 1);
        assert_eq!(report.tempo_events_ignored, 1);
        assert_eq!(report.ignored_events, 1);
        let imported = &project.tracks[0];
        assert_eq!(imported.name, "Bass");
        let part = &imported.midi_parts[0];
        assert_eq!(part.notes[0].channel, 2);
        assert_eq!(part.notes[0].pitch, 60);
        assert!((part.notes[0].duration_beats - 1.0).abs() < 1e-9);
        assert_eq!(part.controller_events[0].controller, 11);
        assert_eq!(part.controller_events[0].value, 64);
        assert_eq!(project.tempo_bpm, 120.0);
    }

    #[test]
    fn imports_format_one_track_names_channels_and_supported_events() {
        let conductor = [
            0x00, 0xff, 0x51, 0x03, 0x06, 0x1a, 0x80, // tempo, ignored on import
            0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, // meter, left unchanged
            0x00, 0xff, 0x2f, 0x00,
        ];
        let piano = [
            0x00, 0xff, 0x03, 0x05, b'P', b'i', b'a', b'n', b'o', 0x00, 0x91, 60, 100, 0x81, 0x70,
            0x81, 60, 0, // channel 1 note-off at tick 240
            0x00, 0xb3, 7, 90, 0x00, 0xe1, 0, 64, // CC plus unsupported pitch bend
            0x00, 0xf0, 0x02, 0x7d, 0x01, // ignored SysEx
            0x00, 0xff, 0x2f, 0x00,
        ];
        let pad = [
            0x00, 0xff, 0x03, 0x03, b'P', b'a', b'd', 0x00, 0x90, 72, 64, 0x83, 0x60, 0x80, 72, 0,
            0x00, 0xff, 0x2f, 0x00,
        ];
        let bytes = format_one_tracks(&[&conductor, &piano, &pad]);
        let mut project = Project::default();
        let report = import_standard_midi(&mut project, &bytes).unwrap();

        assert_eq!(report.tracks_added, 2);
        assert_eq!(report.notes_imported, 2);
        assert_eq!(report.controllers_imported, 1);
        assert_eq!(report.tempo_events_ignored, 1);
        assert_eq!(report.meta_events_ignored, 1);
        assert_eq!(report.sysex_events_ignored, 1);
        assert_eq!(report.unsupported_channel_events, 1);
        assert_eq!(project.tempo_bpm, 120.0);
        assert_eq!(project.beats_per_bar, 4);
        assert_eq!(project.beat_unit, 4);

        let piano = &project.tracks[0];
        assert_eq!(piano.name, "Piano");
        assert_eq!(piano.midi_parts[0].notes[0].channel, 1);
        assert_eq!(piano.midi_parts[0].notes[0].pitch, 60);
        assert!((piano.midi_parts[0].notes[0].duration_beats - 0.5).abs() < 1e-9);
        assert_eq!(piano.midi_parts[0].controller_events[0].channel, 3);
        assert_eq!(piano.midi_parts[0].controller_events[0].controller, 7);
        assert_eq!(piano.midi_parts[0].controller_events[0].value, 90);

        let pad = &project.tracks[1];
        assert_eq!(pad.name, "Pad");
        assert_eq!(pad.midi_parts[0].notes[0].channel, 0);
        assert!((pad.midi_parts[0].notes[0].duration_beats - 1.0).abs() < 1e-9);
    }

    #[test]
    fn export_then_import_preserves_timeline_channels_tempo_and_meter() {
        let original = project_with_midi();
        let (bytes, report) = export_standard_midi(&original).unwrap();
        assert_eq!(read_u16(&bytes, 8).unwrap(), 1);
        assert_eq!(report.tracks_written, 2);
        assert_eq!(report.notes_written, 1);
        assert_eq!(report.controllers_written, 1);
        let mut imported = Project::default();
        import_standard_midi(&mut imported, &bytes).unwrap();
        assert_eq!(imported.tempo_bpm, 120.0); // import leaves destination tempo intact
        assert_eq!(imported.tracks[0].name, "Bass");
        let part = &imported.tracks[0].midi_parts[0];
        assert!((part.notes[0].start_beats - 4.5).abs() < 1e-9);
        assert!((part.notes[0].duration_beats - 1.25).abs() < 1e-9);
        assert_eq!(part.notes[0].channel, 2);
        assert!((part.controller_events[0].position_beats - 6.0).abs() < 1e-9);
        assert_eq!(part.controller_events[0].channel, 3);
    }

    #[test]
    fn a_scoped_export_contains_only_the_chosen_part() {
        let mut project = project_with_midi();
        project.add_track(TrackKind::Midi);
        project.tracks[1].name = "Lead".to_owned();
        let other_part = project.add_midi_part(1, 0.0, 4.0).unwrap();
        project
            .add_midi_note_on_channel(1, other_part, 0.0, 1.0, 72, 0, 100)
            .unwrap();
        let (_, whole) = export_standard_midi(&project).unwrap();
        assert_eq!(whole.tracks_written, 3, "conductor plus two MIDI tracks");
        assert_eq!(whole.notes_written, 2);

        // One part only: the conductor and that one track.
        let (bytes, report) = export_standard_midi_scope(
            &project,
            &MidiExportScope {
                part_ids: vec![other_part],
                ..MidiExportScope::default()
            },
        )
        .unwrap();
        assert_eq!(report.tracks_written, 2);
        assert_eq!(report.notes_written, 1);
        assert_eq!(
            report.controllers_written, 0,
            "the bass part's CC is excluded"
        );
        let mut imported = Project::default();
        import_standard_midi(&mut imported, &bytes).unwrap();
        assert_eq!(imported.tracks.len(), 1);
        assert_eq!(imported.tracks[0].name, "Lead");

        // One track only, with every part on it.
        let (_, report) = export_standard_midi_scope(
            &project,
            &MidiExportScope {
                track_ids: vec![project.tracks[0].id],
                ..MidiExportScope::default()
            },
        )
        .unwrap();
        assert_eq!(report.notes_written, 1);
        assert_eq!(report.controllers_written, 1);

        // A scope that matches nothing still writes only the conductor.
        let (_, report) = export_standard_midi_scope(
            &project,
            &MidiExportScope {
                part_ids: vec![9_999],
                ..MidiExportScope::default()
            },
        )
        .unwrap();
        assert_eq!(report.tracks_written, 1);
        assert_eq!(report.notes_written, 0);
    }

    #[test]
    fn a_loop_range_clips_crossing_notes_and_drops_the_rest() {
        let mut project = Project::default();
        project.tempo_bpm = 120.0;
        project.add_track(TrackKind::Midi);
        let part = project.add_midi_part(0, 0.0, 8.0).unwrap();
        // Before, crossing, inside, crossing the end, and after the 2..4 range.
        for (start, duration, pitch) in [
            (0.0, 1.0, 60),
            (1.5, 1.0, 62),
            (2.5, 0.5, 64),
            (3.5, 1.0, 65),
            (5.0, 1.0, 67),
        ] {
            project
                .add_midi_note_on_channel(0, part, start, duration, pitch, 0, 100)
                .unwrap();
        }
        project
            .add_midi_controller_event_on_channel(0, part, 1.0, 11, 0, 10)
            .unwrap();
        project
            .add_midi_controller_event_on_channel(0, part, 3.0, 11, 0, 20)
            .unwrap();

        let (bytes, report) = export_standard_midi_scope(
            &project,
            &MidiExportScope {
                start_beats: Some(2.0),
                end_beats: Some(4.0),
                ..MidiExportScope::default()
            },
        )
        .unwrap();
        assert_eq!(report.notes_written, 3, "the crossing and inside notes");
        assert_eq!(
            report.controllers_written, 1,
            "only the CC inside the range"
        );

        let mut imported = Project::default();
        import_standard_midi(&mut imported, &bytes).unwrap();
        let exported = &imported.tracks[0].midi_parts[0];
        let spans = exported
            .notes
            .iter()
            .map(|note| {
                (
                    note.start_beats,
                    note.start_beats + note.duration_beats,
                    note.pitch,
                )
            })
            .collect::<Vec<_>>();
        // The note from 1.5 is clipped to start at 2.0, the note from 3.5 is
        // clipped to end at 4.0, and the inside note is untouched.
        assert!(spans.iter().any(|(start, end, pitch)| *pitch == 62
            && (*start - 2.0).abs() < 1e-6
            && (*end - 2.5).abs() < 1e-6));
        assert!(spans.iter().any(|(start, end, pitch)| *pitch == 64
            && (*start - 2.5).abs() < 1e-6
            && (*end - 3.0).abs() < 1e-6));
        assert!(spans.iter().any(|(start, end, pitch)| *pitch == 65
            && (*start - 3.5).abs() < 1e-6
            && (*end - 4.0).abs() < 1e-6));
        assert!(
            !spans
                .iter()
                .any(|(_, _, pitch)| *pitch == 60 || *pitch == 67)
        );

        // An unusable range is ignored rather than exporting nothing.
        let (_, report) = export_standard_midi_scope(
            &project,
            &MidiExportScope {
                start_beats: Some(4.0),
                end_beats: Some(4.0),
                ..MidiExportScope::default()
            },
        )
        .unwrap();
        assert_eq!(report.notes_written, 5);
        assert!(!MidiExportScope::default().is_whole_project() == false);
        assert!(
            MidiExportScope {
                part_ids: vec![part],
                ..MidiExportScope::default()
            }
            .is_whole_project()
                == false
        );
    }

    #[test]
    fn export_matches_independent_standard_midi_event_decode() {
        let original = project_with_midi();
        let (bytes, _) = export_standard_midi(&original).unwrap();
        let (format, division, tracks) = independently_decode_export(&bytes);

        assert_eq!(format, 1);
        assert_eq!(division, 480);
        assert_eq!(tracks.len(), 2);
        assert_eq!(
            tracks[0],
            vec![
                (0, [vec![0xff, 0x03], b"OpenTrack Tempo".to_vec()].concat()),
                (0, vec![0xff, 0x51, 0x07, 0x71, 0x7d]),
                (0, vec![0xff, 0x58, 0x07, 0x03, 24, 8]),
                (0, vec![0xff, 0x2f]),
            ]
        );
        assert_eq!(
            tracks[1],
            vec![
                (0, vec![0xff, 0x03, b'B', b'a', b's', b's']),
                (2160, vec![0x92, 60, 96]),
                (2760, vec![0x82, 60, 0]),
                (2880, vec![0xb3, 11, 81]),
                (2880, vec![0xff, 0x2f]),
            ]
        );
    }

    #[test]
    fn malformed_or_unsupported_file_does_not_mutate_the_project() {
        let mut project = project_with_midi();
        let before = project.clone();
        assert!(matches!(
            import_standard_midi(&mut project, b"not midi"),
            Err(MidiFileError::InvalidHeader)
        ));
        assert_eq!(project, before);

        let mut bytes = format_zero_track(&[0x00, 0xff, 0x2f, 0x00]);
        bytes[9] = 2; // format 2 is not supported
        assert!(matches!(
            import_standard_midi(&mut project, &bytes),
            Err(MidiFileError::UnsupportedFormat(2))
        ));
        assert_eq!(project, before);
    }

    #[test]
    fn overlapping_same_key_notes_pair_fifo_and_velocity_zero_is_note_off() {
        let bytes = format_zero_track(&[
            0x00, 0x90, 60, 50, // first note begins
            0x83, 0x60, 60, 80, // second begins one beat later by running status
            0x83, 0x60, 60, 0, // first ends one beat later; velocity-zero note-on
            0x83, 0x60, 0x80, 60, 0, // second ends one beat later
            0x00, 0xff, 0x2f, 0x00,
        ]);
        let mut project = Project::default();
        import_standard_midi(&mut project, &bytes).unwrap();
        let notes = &project.tracks[0].midi_parts[0].notes;
        assert_eq!(notes.len(), 2);
        assert!((notes[0].start_beats - 0.0).abs() < 1e-9);
        assert!((notes[0].duration_beats - 2.0).abs() < 1e-9);
        assert!((notes[1].start_beats - 1.0).abs() < 1e-9);
        assert!((notes[1].duration_beats - 2.0).abs() < 1e-9);
        assert_eq!(notes[0].velocity, 50);
        assert_eq!(notes[1].velocity, 80);
    }

    #[test]
    fn rejects_smpte_division() {
        let mut bytes = format_zero_track(&[0x00, 0xff, 0x2f, 0x00]);
        bytes[12] = 0xe7;
        bytes[13] = 0x28;
        let mut project = Project::default();
        assert_eq!(
            import_standard_midi(&mut project, &bytes),
            Err(MidiFileError::UnsupportedDivision)
        );
        assert!(project.tracks.is_empty());
    }

    #[test]
    fn closes_open_notes_at_end_and_reports_coalesced_controller_points() {
        let bytes = format_zero_track(&[
            0x00, 0x90, 64, 90, 0x00, 0xb0, 1, 20, 0x00, 0xb0, 1, 100, 0x83, 0x60, 0xff, 0x2f,
            0x00, // end one beat later
        ]);
        let mut project = Project::default();
        let report = import_standard_midi(&mut project, &bytes).unwrap();
        let part = &project.tracks[0].midi_parts[0];
        assert_eq!(part.notes.len(), 1);
        assert!((part.notes[0].duration_beats - 1.0).abs() < 1e-9);
        assert_eq!(part.controller_events.len(), 1);
        assert_eq!(part.controller_events[0].value, 100);
        assert_eq!(report.controllers_imported, 1);
        assert_eq!(report.duplicate_controller_events, 1);
        assert_eq!(report.ignored_events, 1);
    }
}
