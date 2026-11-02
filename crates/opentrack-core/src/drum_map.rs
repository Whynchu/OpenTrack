//! Project-owned drum-map presentation for MIDI and Instrument tracks.
//!
//! The first version changes only what names and ordering an editor displays;
//! it never changes sounding pitch. An empty map is the identity presentation:
//! rows fall back to the MIDI note number, so no General MIDI naming is implied.

use serde::{Deserialize, Serialize};

/// Maximum rows in one drum map (the MIDI pitch range).
pub const MAX_DRUM_MAP_ROWS: usize = 128;
/// Maximum characters in a drum-map name.
pub const MAX_DRUM_MAP_NAME_CHARS: usize = 48;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct DrumMap {
    /// Row order is display order, first at the top or bottom according to the
    /// consuming editor; the model does not imply a direction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    rows: Vec<DrumMapRow>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DrumMapRow {
    pub pitch: u8,
    pub name: String,
}

impl DrumMap {
    /// An empty map: every pitch displays as its own MIDI number.
    pub fn identity() -> Self {
        Self::default()
    }

    pub fn rows(&self) -> &[DrumMapRow] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The configured name for `pitch`, or `None` for the identity fallback.
    pub fn name_for(&self, pitch: u8) -> Option<&str> {
        self.rows
            .iter()
            .find(|row| row.pitch == pitch)
            .map(|row| row.name.as_str())
    }

    /// Replaces the whole map after validation. An empty slice restores identity.
    pub fn set_rows(&mut self, rows: &[(u8, String)]) -> Result<(), String> {
        if rows.len() > MAX_DRUM_MAP_ROWS {
            return Err(format!("a drum map holds at most {MAX_DRUM_MAP_ROWS} rows"));
        }
        let mut seen = [false; 256];
        let mut validated = Vec::with_capacity(rows.len());
        for (pitch, name) in rows {
            if seen[usize::from(*pitch)] {
                return Err(format!("pitch {pitch} appears more than once"));
            }
            seen[usize::from(*pitch)] = true;
            let name = name.trim();
            if name.is_empty() {
                return Err(format!("pitch {pitch} has an empty name"));
            }
            if name.chars().count() > MAX_DRUM_MAP_NAME_CHARS {
                return Err(format!(
                    "pitch {pitch} name is longer than {MAX_DRUM_MAP_NAME_CHARS} characters"
                ));
            }
            if name.chars().any(char::is_control) {
                return Err(format!("pitch {pitch} name contains a control character"));
            }
            validated.push(DrumMapRow {
                pitch: *pitch,
                name: name.to_owned(),
            });
        }
        self.rows = validated;
        Ok(())
    }

    /// Validates data loaded from a project without mutating it.
    pub fn validate(&self) -> Result<(), String> {
        if self.rows.len() > MAX_DRUM_MAP_ROWS {
            return Err(format!("a drum map holds at most {MAX_DRUM_MAP_ROWS} rows"));
        }
        let mut seen = [false; 256];
        for row in &self.rows {
            if seen[usize::from(row.pitch)] {
                return Err(format!("pitch {} appears more than once", row.pitch));
            }
            seen[usize::from(row.pitch)] = true;
            let name = row.name.trim();
            if name.is_empty() {
                return Err(format!("pitch {} has an empty name", row.pitch));
            }
            if name.chars().count() > MAX_DRUM_MAP_NAME_CHARS {
                return Err(format!(
                    "pitch {} name is longer than {MAX_DRUM_MAP_NAME_CHARS} characters",
                    row.pitch
                ));
            }
            if name.chars().any(char::is_control) {
                return Err(format!(
                    "pitch {} name contains a control character",
                    row.pitch
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_has_no_rows_and_falls_back_to_numbers() {
        let map = DrumMap::identity();
        assert!(map.is_empty());
        assert_eq!(map.name_for(36), None);
        assert!(map.validate().is_ok());
    }

    #[test]
    fn a_valid_map_preserves_display_order_and_round_trips() {
        let mut map = DrumMap::default();
        map.set_rows(&[
            (38, "Snare".to_owned()),
            (36, "Kick".to_owned()),
            (42, "Closed HH".to_owned()),
        ])
        .expect("map");
        assert_eq!(
            map.rows().iter().map(|row| row.pitch).collect::<Vec<_>>(),
            vec![38, 36, 42]
        );
        assert_eq!(map.name_for(36), Some("Kick"));
        let encoded = serde_json::to_string(&map).expect("encode");
        let decoded: DrumMap = serde_json::from_str(&encoded).expect("decode");
        assert_eq!(decoded, map);
        assert!(decoded.validate().is_ok());
    }

    #[test]
    fn duplicate_pitches_and_bad_names_are_refused_without_changing_the_map() {
        let mut map = DrumMap::default();
        map.set_rows(&[(36, "Kick".to_owned())]).expect("map");
        for bad in [
            vec![(36, "A".to_owned()), (36, "B".to_owned())],
            vec![(37, "  ".to_owned())],
            vec![(37, "x".repeat(MAX_DRUM_MAP_NAME_CHARS + 1))],
            vec![(37, "bad\nname".to_owned())],
        ] {
            assert!(map.set_rows(&bad).is_err(), "{bad:?}");
        }
        assert_eq!(map.rows().len(), 1, "a refused map leaves the old one");
    }

    #[test]
    fn malformed_loaded_rows_are_reported() {
        let encoded = r#"{"rows":[{"pitch":36,"name":"Kick"},{"pitch":36,"name":"Snare"}]}"#;
        let map: DrumMap = serde_json::from_str(encoded).expect("decode");
        assert_eq!(
            map.validate(),
            Err("pitch 36 appears more than once".to_owned())
        );
    }

    #[test]
    fn an_empty_slice_restores_identity() {
        let mut map = DrumMap::default();
        map.set_rows(&[(36, "Kick".to_owned())]).expect("map");
        map.set_rows(&[]).expect("clear");
        assert!(map.is_empty());
    }
}
