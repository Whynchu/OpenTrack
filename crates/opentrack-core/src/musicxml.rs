//! MusicXML note import for MIDI tracks.
//!
//! This is a deliberately small reader for the partwise MusicXML shape: it reads
//! part names, divisions, notes, rests, chords, backups/forwards and a tempo hint.
//! It is not a general score importer: articulations, ties, slurs, grace notes,
//! directions and layout are outside the scope and are reported as unsupported
//! rather than guessed at.

use std::collections::BTreeMap;

/// Largest MusicXML document accepted, so a mistaken file cannot stall the UI.
pub const MAX_MUSICXML_BYTES: usize = 8 * 1024 * 1024;
/// Bound on imported parts.
pub const MAX_MUSICXML_PARTS: usize = 64;
/// Bound on imported notes across all parts.
pub const MAX_MUSICXML_NOTES: usize = 100_000;

/// One note ready to become a MIDI note.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicXmlNote {
    pub start_beats: f64,
    pub duration_beats: f64,
    pub pitch: u8,
    pub velocity: u8,
}

/// One score part ready to become a MIDI part.
#[derive(Clone, Debug, PartialEq)]
pub struct MusicXmlPart {
    pub name: String,
    pub start_beats: f64,
    pub duration_beats: f64,
    pub notes: Vec<MusicXmlNote>,
}

/// The parsed document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MusicXmlImport {
    pub parts: Vec<MusicXmlPart>,
    /// Tempo hint from `<sound tempo="...">`, if the file carries one.
    pub tempo_bpm: Option<f64>,
    /// Elements that were present but outside the supported scope.
    pub unsupported: Vec<&'static str>,
}

impl MusicXmlImport {
    pub fn note_count(&self) -> usize {
        self.parts.iter().map(|part| part.notes.len()).sum()
    }
}

struct PartState {
    name: String,
    divisions: f64,
    cursor: f64,
    last_note_start: f64,
    notes: Vec<MusicXmlNote>,
}

/// Reads a partwise MusicXML document.
pub fn import_musicxml(bytes: &[u8]) -> Result<MusicXmlImport, String> {
    if bytes.is_empty() {
        return Err("the MusicXML file is empty".to_owned());
    }
    if bytes.len() > MAX_MUSICXML_BYTES {
        return Err("the MusicXML file is larger than the 8 MiB import limit".to_owned());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "the MusicXML file is not valid UTF-8".to_owned())?;
    let mut scanner = Scanner::new(text);
    let mut part_names: BTreeMap<String, String> = BTreeMap::new();
    let mut states: BTreeMap<String, PartState> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut current_part: Option<String> = None;
    let mut current_note: Option<NoteDraft> = None;
    let mut tempo_bpm = None;
    let mut unsupported = Vec::new();
    let mut in_pitch = false;
    let mut in_rest = false;

    while let Some(token) = scanner.next() {
        match token {
            Token::Open {
                name,
                attributes,
                self_closing,
            } => match name.as_str() {
                "score-part" => {
                    if let Some(id) = attributes.get("id") {
                        current_part = Some(id.clone());
                        part_names.entry(id.clone()).or_default();
                    }
                }
                "part-name" => {
                    if let Some(id) = current_part.clone()
                        && let Some(value) = scanner.text_until_close("part-name")?
                    {
                        part_names.insert(id, value.trim().to_owned());
                    }
                }
                "part" => {
                    let Some(id) = attributes.get("id").cloned() else {
                        return Err("a <part> element has no id".to_owned());
                    };
                    if order.len() >= MAX_MUSICXML_PARTS {
                        return Err("the file has more parts than the import supports".to_owned());
                    }
                    order.push(id.clone());
                    states.entry(id.clone()).or_insert(PartState {
                        name: String::new(),
                        divisions: 1.0,
                        cursor: 0.0,
                        last_note_start: 0.0,
                        notes: Vec::new(),
                    });
                    current_part = Some(id);
                }
                "divisions" => {
                    if let Some(part) = current_part.clone()
                        && let Some(value) = scanner.text_until_close("divisions")?
                        && let Ok(divisions) = value.trim().parse::<f64>()
                        && divisions.is_finite()
                        && divisions > 0.0
                    {
                        if let Some(state) = states.get_mut(&part) {
                            state.divisions = divisions;
                        }
                    }
                }
                "note" => {
                    if !self_closing {
                        current_note = Some(NoteDraft::default());
                        in_pitch = false;
                        in_rest = false;
                    }
                }
                "rest" => {
                    in_rest = true;
                    if let Some(note) = current_note.as_mut() {
                        note.rest = true;
                    }
                }
                "chord" => {
                    if let Some(note) = current_note.as_mut() {
                        note.chord = true;
                    }
                }
                "pitch" => in_pitch = true,
                "step" => {
                    if in_pitch
                        && let Some(value) = scanner.text_until_close("step")?
                        && let Some(note) = current_note.as_mut()
                    {
                        note.step = value.trim().chars().next();
                    }
                }
                "octave" => {
                    if in_pitch
                        && let Some(value) = scanner.text_until_close("octave")?
                        && let Some(note) = current_note.as_mut()
                    {
                        note.octave = value.trim().parse::<i16>().ok();
                    }
                }
                "alter" => {
                    if in_pitch
                        && let Some(value) = scanner.text_until_close("alter")?
                        && let Some(note) = current_note.as_mut()
                    {
                        note.alter = value.trim().parse::<f64>().unwrap_or(0.0);
                    }
                }
                "duration" => {
                    if let Some(value) = scanner.text_until_close("duration")?
                        && let Some(note) = current_note.as_mut()
                    {
                        note.duration = value.trim().parse::<f64>().ok();
                    }
                }
                "backup" | "forward" => {
                    // The amount lives in a child <duration>, so the element's
                    // inner text is read and the duration extracted from it.
                    let inner = scanner.text_until_close(&name)?.unwrap_or_default();
                    let amount = duration_within(&inner).unwrap_or(0.0);
                    if let Some(part) = current_part.clone()
                        && let Some(state) = states.get_mut(&part)
                    {
                        let beats = amount / state.divisions.max(1.0);
                        if name == "backup" {
                            state.cursor = (state.cursor - beats).max(0.0);
                            if state.cursor < -1e-9 {
                                unsupported.push("backup before the part start");
                            }
                        } else {
                            state.cursor += beats;
                        }
                    }
                }
                "sound" => {
                    if let Some(tempo) = attributes
                        .get("tempo")
                        .and_then(|value| value.parse::<f64>().ok())
                        .filter(|value| (1.0..=1000.0).contains(value))
                    {
                        tempo_bpm = Some(tempo);
                    }
                }
                "grace" => {
                    unsupported.push("grace notes");
                    if let Some(note) = current_note.as_mut() {
                        note.grace = true;
                    }
                }
                "tie" | "tied" => unsupported.push("ties"),
                "slur" => unsupported.push("slurs"),
                "articulations" => unsupported.push("articulations"),
                _ => {}
            },
            Token::Close { name } => match name.as_str() {
                "note" => {
                    if let Some(draft) = current_note.take()
                        && !draft.grace
                    {
                        if let Some(part) = current_part.clone()
                            && let Some(state) = states.get_mut(&part)
                        {
                            let duration = draft.duration.unwrap_or(0.0) / state.divisions.max(1.0);
                            if draft.rest {
                                state.cursor += duration.max(0.0);
                            } else {
                                let start = if draft.chord {
                                    state.last_note_start
                                } else {
                                    let start = state.cursor;
                                    state.last_note_start = start;
                                    state.cursor += duration.max(0.0);
                                    start
                                };
                                if let Some(pitch) = draft.pitch() {
                                    if duration > 0.0 {
                                        state.notes.push(MusicXmlNote {
                                            start_beats: start,
                                            duration_beats: duration,
                                            pitch,
                                            velocity: 96,
                                        });
                                    }
                                } else {
                                    unsupported.push("notes without a pitch");
                                }
                            }
                        }
                    }
                }
                "pitch" => in_pitch = false,
                "rest" => in_rest = false,
                "part" => current_part = None,
                _ => {}
            },
            Token::Text(_) => {
                let _ = in_rest;
            }
        }
    }

    let mut parts = Vec::new();
    for id in order {
        let Some(mut state) = states.remove(&id) else {
            continue;
        };
        if state.notes.is_empty() {
            continue;
        }
        state.notes.sort_by(|left, right| {
            left.start_beats
                .total_cmp(&right.start_beats)
                .then(left.pitch.cmp(&right.pitch))
        });
        let start = state
            .notes
            .iter()
            .map(|note| note.start_beats)
            .fold(f64::INFINITY, f64::min);
        let end = state
            .notes
            .iter()
            .map(|note| note.start_beats + note.duration_beats)
            .fold(0.0_f64, f64::max);
        let name = part_names
            .remove(&id)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("Part {id}"));
        parts.push(MusicXmlPart {
            name,
            start_beats: start.max(0.0),
            duration_beats: (end - start).max(1.0 / 64.0),
            notes: state
                .notes
                .into_iter()
                .map(|note| MusicXmlNote {
                    start_beats: note.start_beats - start,
                    ..note
                })
                .collect(),
        });
    }
    if parts.is_empty() {
        return Err("no pitched notes were found in the MusicXML file".to_owned());
    }
    if parts.iter().map(|part| part.notes.len()).sum::<usize>() > MAX_MUSICXML_NOTES {
        return Err("the file has more notes than the import supports".to_owned());
    }
    unsupported.sort_unstable();
    unsupported.dedup();
    Ok(MusicXmlImport {
        parts,
        tempo_bpm,
        unsupported,
    })
}

#[derive(Default)]
struct NoteDraft {
    step: Option<char>,
    octave: Option<i16>,
    alter: f64,
    duration: Option<f64>,
    rest: bool,
    chord: bool,
    grace: bool,
}

impl NoteDraft {
    fn pitch(&self) -> Option<u8> {
        let step = self.step?.to_ascii_uppercase();
        let semitone = match step {
            'C' => 0,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return None,
        };
        let octave = self.octave?;
        let value = (i32::from(octave) + 1) * 12 + semitone + self.alter.round() as i32;
        (0..=127).contains(&value).then_some(value as u8)
    }
}

struct Scanner<'a> {
    text: &'a str,
    cursor: usize,
}

enum Token {
    Open {
        name: String,
        attributes: BTreeMap<String, String>,
        self_closing: bool,
    },
    Close {
        name: String,
    },
    Text(String),
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, cursor: 0 }
    }

    fn next(&mut self) -> Option<Token> {
        while let Some(relative) = self.text[self.cursor..].find('<') {
            let start = self.cursor + relative;
            if start > self.cursor {
                let text = self.text[self.cursor..start].to_owned();
                self.cursor = start;
                return Some(Token::Text(text));
            }
            let end = self.text[start..].find('>')? + start;
            let inner = self.text[start + 1..end].trim();
            self.cursor = end + 1;
            if let Some(rest) = inner.strip_prefix('?') {
                let _ = rest;
                continue; // XML declaration.
            }
            if inner.starts_with('!') {
                continue; // Comments and doctypes.
            }
            if let Some(name) = inner.strip_prefix('/') {
                return Some(Token::Close {
                    name: name.trim().to_owned(),
                });
            }
            let self_closing = inner.ends_with('/');
            let inner = inner.trim_end_matches('/').trim();
            let mut pieces = inner.splitn(2, char::is_whitespace);
            let name = pieces.next().unwrap_or_default().to_owned();
            let mut attributes = BTreeMap::new();
            if let Some(rest) = pieces.next() {
                let mut rest = rest.trim();
                while let Some(equals) = rest.find('=') {
                    let key = rest[..equals].trim().to_owned();
                    rest = rest[equals + 1..].trim_start();
                    let (value, tail) = if let Some(stripped) = rest.strip_prefix('"') {
                        match stripped.find('"') {
                            Some(close) => (stripped[..close].to_owned(), &stripped[close + 1..]),
                            None => (stripped.to_owned(), ""),
                        }
                    } else if let Some(stripped) = rest.strip_prefix('\'') {
                        match stripped.find('\'') {
                            Some(close) => (stripped[..close].to_owned(), &stripped[close + 1..]),
                            None => (stripped.to_owned(), ""),
                        }
                    } else {
                        (String::new(), rest)
                    };
                    attributes.insert(key, decode_entities(&value));
                    rest = tail.trim();
                }
            }
            return Some(Token::Open {
                name,
                attributes,
                self_closing,
            });
        }
        None
    }

    /// Reads text up to the matching close tag, used for leaf elements.
    fn text_until_close(&mut self, tag: &str) -> Result<Option<String>, String> {
        let close = format!("</{tag}>");
        let Some(relative) = self.text[self.cursor..].find(&close) else {
            return Err(format!("the MusicXML file has an unclosed <{tag}> element"));
        };
        let end = self.cursor + relative;
        let value = decode_entities(self.text[self.cursor..end].trim());
        self.cursor = end + close.len();
        Ok(Some(value))
    }
}

fn duration_within(inner: &str) -> Option<f64> {
    let open = inner.find("<duration>")? + "<duration>".len();
    let close = inner[open..].find("</duration>")? + open;
    inner[open..close].trim().parse::<f64>().ok()
}

fn decode_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO_PARTS: &str = r#"<?xml version="1.0"?>
<score-partwise version="4.0">
  <part-list>
    <score-part id="P1"><part-name>Piano</part-name></score-part>
    <score-part id="P2"><part-name>Bass</part-name></score-part>
  </part-list>
  <part id="P1">
    <measure>
      <attributes><divisions>2</divisions></attributes>
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration></note>
      <note><chord/><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration></note>
      <note><rest/><duration>2</duration></note>
      <note><pitch><step>G</step><alter>1</alter><octave>4</octave></pitch><duration>4</duration></note>
      <backup><duration>2</duration></backup>
      <note><pitch><step>B</step><octave>3</octave></pitch><duration>1</duration></note>
      <sound tempo="132"/>
    </measure>
  </part>
  <part id="P2">
    <measure>
      <attributes><divisions>2</divisions></attributes>
      <note><pitch><step>C</step><octave>2</octave></pitch><duration>8</duration></note>
    </measure>
  </part>
</score-partwise>"#;

    #[test]
    fn imports_parts_notes_chords_rests_and_backups() {
        let import = import_musicxml(TWO_PARTS.as_bytes()).expect("import");
        assert_eq!(import.parts.len(), 2);
        assert_eq!(import.parts[0].name, "Piano");
        assert_eq!(import.parts[1].name, "Bass");
        assert_eq!(import.tempo_bpm, Some(132.0));
        let piano = &import.parts[0];
        // C4 and E4 start together (chord), a rest takes one beat, then G#4.
        let pitches = piano
            .notes
            .iter()
            .map(|note| note.pitch)
            .collect::<Vec<_>>();
        assert_eq!(pitches, vec![60, 64, 68, 59], "{pitches:?}");
        assert_eq!(piano.notes[0].start_beats, 0.0);
        assert_eq!(piano.notes[1].start_beats, 0.0);
        assert_eq!(piano.notes[0].duration_beats, 1.0);
        assert_eq!(piano.notes[2].start_beats, 2.0);
        assert_eq!(piano.notes[2].duration_beats, 2.0);
        // The backup removes one beat from the four the notes advanced, so B3
        // starts at beat 3 inside the G#4's span.
        assert!(
            (piano.notes[3].start_beats - 3.0).abs() < 1e-9,
            "{:?}",
            piano.notes[3]
        );
        assert_eq!(piano.notes[3].duration_beats, 0.5);
        // The part spans from its first to its last note.
        assert!((piano.duration_beats - 4.0).abs() < 1e-9);
        // The second part's notes are relative to its own start.
        assert_eq!(import.parts[1].notes[0].start_beats, 0.0);
        assert_eq!(import.parts[1].notes[0].pitch, 36);
        assert_eq!(import.parts[1].notes[0].duration_beats, 4.0);
        assert_eq!(import.note_count(), 5);
    }

    #[test]
    fn rejects_unnoteable_or_malformed_documents() {
        assert!(import_musicxml(b"").is_err());
        assert!(import_musicxml(b"not xml").is_err());
        assert!(
            import_musicxml(b"<score-partwise><part id=\"P1\"></part></score-partwise>").is_err()
        );
        assert!(
            import_musicxml(b"<score-partwise><part id=\"P1\"><note><pitch><step>C</step>")
                .is_err()
        );
        let rest_only = br#"<score-partwise><part id="P1"><note><rest/><duration>4</duration></note></part></score-partwise>"#;
        assert!(import_musicxml(rest_only).is_err(), "no pitched notes");
        // Over-size input is refused before parsing.
        let huge = vec![b' '; MAX_MUSICXML_BYTES + 1];
        assert!(import_musicxml(&huge).is_err());
    }

    #[test]
    fn unsupported_elements_are_reported_rather_than_guessed() {
        let with_ties = br#"<score-partwise><part id="P1">
            <note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration><tie type="start"/></note>
            <note><grace/><pitch><step>D</step><octave>4</octave></pitch></note>
            <note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration></note>
        </part></score-partwise>"#;
        let import = import_musicxml(with_ties).expect("import");
        assert!(import.unsupported.contains(&"ties"));
        assert!(import.unsupported.contains(&"grace notes"));
        // The grace note is not imported; the two real notes are.
        assert_eq!(import.note_count(), 2);
        assert_eq!(import.parts[0].notes[1].pitch, 64);
    }
}
