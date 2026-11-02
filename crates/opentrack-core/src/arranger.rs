//! Named arranger sections and their ordered play sequence.
//!
//! Sections are half-open beat ranges that reference existing timeline content;
//! editing the arranger never moves or copies source clips. The sequence is a
//! flat list of section references with positive repeat counts. There is no
//! nested arranger, so recursion cannot be expressed.

use serde::{Deserialize, Serialize};

pub const MAX_ARRANGER_SECTIONS: usize = 128;
pub const MAX_ARRANGER_SEQUENCE_ITEMS: usize = 1024;
/// Total section plays after repeats, including repeated entries.
pub const MAX_EXPANDED_ARRANGER_ITEMS: usize = 1024;
pub const MAX_ARRANGER_NAME_CHARS: usize = 96;
pub const MAX_ARRANGER_BEATS: f64 = 1_000_000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArrangerSection {
    pub id: u64,
    pub name: String,
    pub start_beats: f64,
    pub end_beats: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArrangerSequenceItem {
    pub section_id: u64,
    pub repeats: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Arranger {
    pub sections: Vec<ArrangerSection>,
    pub sequence: Vec<ArrangerSequenceItem>,
    pub next_section_id: u64,
}

impl Default for Arranger {
    fn default() -> Self {
        Self {
            sections: Vec::new(),
            sequence: Vec::new(),
            next_section_id: 1,
        }
    }
}

fn valid_range(start_beats: f64, end_beats: f64) -> bool {
    start_beats.is_finite()
        && end_beats.is_finite()
        && start_beats >= 0.0
        && end_beats > start_beats + 1.0e-9
        && end_beats <= MAX_ARRANGER_BEATS
}

fn valid_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name.chars().count() <= MAX_ARRANGER_NAME_CHARS
}

impl Arranger {
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty() && self.sequence.is_empty()
    }

    pub fn section(&self, id: u64) -> Option<&ArrangerSection> {
        self.sections.iter().find(|section| section.id == id)
    }

    pub fn add_section(
        &mut self,
        name: impl Into<String>,
        start_beats: f64,
        end_beats: f64,
    ) -> Option<u64> {
        if self.sections.len() >= MAX_ARRANGER_SECTIONS
            || self.next_section_id == 0
            || self.next_section_id == u64::MAX
            || !valid_range(start_beats, end_beats)
        {
            return None;
        }
        let name = name.into();
        let name = name.trim();
        if !valid_name(name) {
            return None;
        }
        let id = self.next_section_id;
        self.next_section_id += 1;
        self.sections.push(ArrangerSection {
            id,
            name: name.chars().take(MAX_ARRANGER_NAME_CHARS).collect(),
            start_beats,
            end_beats,
        });
        self.sections
            .sort_by(|left, right| left.start_beats.total_cmp(&right.start_beats));
        Some(id)
    }

    pub fn rename_section(&mut self, id: u64, name: impl Into<String>) -> bool {
        let name = name.into();
        let name = name.trim();
        if !valid_name(name) {
            return false;
        }
        let Some(section) = self.sections.iter_mut().find(|section| section.id == id) else {
            return false;
        };
        section.name = name.chars().take(MAX_ARRANGER_NAME_CHARS).collect();
        true
    }

    pub fn set_section_range(&mut self, id: u64, start_beats: f64, end_beats: f64) -> bool {
        if !valid_range(start_beats, end_beats) {
            return false;
        }
        let Some(section) = self.sections.iter_mut().find(|section| section.id == id) else {
            return false;
        };
        section.start_beats = start_beats;
        section.end_beats = end_beats;
        self.sections
            .sort_by(|left, right| left.start_beats.total_cmp(&right.start_beats));
        true
    }

    /// Removes a section and every sequence reference to it in one operation.
    pub fn remove_section(&mut self, id: u64) -> bool {
        let before = self.sections.len();
        self.sections.retain(|section| section.id != id);
        if self.sections.len() == before {
            return false;
        }
        self.sequence.retain(|item| item.section_id != id);
        true
    }

    pub fn append_sequence(&mut self, section_id: u64, repeats: u32) -> bool {
        if repeats == 0
            || self.section(section_id).is_none()
            || self.sequence.len() >= MAX_ARRANGER_SEQUENCE_ITEMS
            || self
                .sequence
                .iter()
                .map(|item| u64::from(item.repeats))
                .sum::<u64>()
                .saturating_add(u64::from(repeats))
                > MAX_EXPANDED_ARRANGER_ITEMS as u64
        {
            return false;
        }
        self.sequence.push(ArrangerSequenceItem {
            section_id,
            repeats,
        });
        true
    }

    pub fn set_sequence_repeats(&mut self, index: usize, repeats: u32) -> bool {
        let Some(item) = self.sequence.get(index) else {
            return false;
        };
        if repeats == 0 {
            return false;
        }
        let others = self
            .sequence
            .iter()
            .enumerate()
            .filter(|(candidate, _)| *candidate != index)
            .map(|(_, item)| u64::from(item.repeats))
            .sum::<u64>();
        if others + u64::from(repeats) > MAX_EXPANDED_ARRANGER_ITEMS as u64 {
            return false;
        }
        let section_id = item.section_id;
        self.sequence[index] = ArrangerSequenceItem {
            section_id,
            repeats,
        };
        true
    }

    pub fn move_sequence_item(&mut self, from: usize, to: usize) -> bool {
        if from >= self.sequence.len() || to >= self.sequence.len() || from == to {
            return false;
        }
        let item = self.sequence.remove(from);
        self.sequence.insert(to, item);
        true
    }

    pub fn remove_sequence_item(&mut self, index: usize) -> bool {
        if index >= self.sequence.len() {
            return false;
        }
        self.sequence.remove(index);
        true
    }

    /// Total playback length of the sequence, or `None` when it has no items or
    /// references an unknown section.
    pub fn expanded_duration_beats(&self) -> Option<f64> {
        let mut total = 0.0;
        for item in &self.sequence {
            let section = self.section(item.section_id)?;
            let length = section.end_beats - section.start_beats;
            if !length.is_finite() || length <= 0.0 || item.repeats == 0 {
                return None;
            }
            total += length * f64::from(item.repeats);
        }
        total.is_finite().then_some(total)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.sections.len() > MAX_ARRANGER_SECTIONS {
            return Err("an arranger cannot contain more than 128 sections");
        }
        if self.sequence.len() > MAX_ARRANGER_SEQUENCE_ITEMS {
            return Err("an arranger sequence cannot exceed 1024 items");
        }
        if self.next_section_id == 0
            || self
                .sections
                .iter()
                .any(|section| section.id >= self.next_section_id)
        {
            return Err("arranger section IDs must be nonzero, unique and below the next ID");
        }
        let mut ids = std::collections::BTreeSet::new();
        for section in &self.sections {
            if section.id == 0 || !ids.insert(section.id) {
                return Err("arranger section IDs must be nonzero and unique");
            }
            if !valid_range(section.start_beats, section.end_beats) || !valid_name(&section.name) {
                return Err("an arranger section has an unusable name or beat range");
            }
        }
        let mut expanded = 0u64;
        for item in &self.sequence {
            if item.repeats == 0 || !ids.contains(&item.section_id) {
                return Err(
                    "arranger sequence items must reference a section with positive repeats",
                );
            }
            expanded = expanded.saturating_add(u64::from(item.repeats));
            if expanded > MAX_EXPANDED_ARRANGER_ITEMS as u64 {
                return Err("the arranger sequence expands to more than 1024 plays");
            }
        }
        if let Some(duration) = self.expanded_duration_beats()
            && duration > MAX_ARRANGER_BEATS
        {
            return Err("the expanded arranger sequence is too long");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_and_sequence_preserve_stable_ids_and_sorted_sections() {
        let mut arranger = Arranger {
            next_section_id: 1,
            ..Arranger::default()
        };
        let first = arranger.add_section("Intro", 0.0, 8.0).expect("section");
        let second = arranger.add_section("Verse", 4.0, 12.0).expect("section");
        assert_eq!(arranger.sections[0].id, first);
        assert_eq!(arranger.sections[1].id, second);
        assert!(arranger.rename_section(first, "Opening"));
        assert!(arranger.set_section_range(first, 0.0, 4.0));
        assert!(arranger.append_sequence(second, 2));
        assert!(arranger.append_sequence(first, 1));
        assert_eq!(arranger.expanded_duration_beats(), Some(20.0));
        assert!(arranger.move_sequence_item(0, 1));
        assert_eq!(arranger.sequence[1].section_id, second);
        assert!(arranger.set_sequence_repeats(1, 3));
        assert_eq!(arranger.expanded_duration_beats(), Some(28.0));
        assert_eq!(arranger.validate(), Ok(()));
    }

    #[test]
    fn removal_drops_references_and_invalid_inputs_are_refused() {
        let mut arranger = Arranger {
            next_section_id: 1,
            ..Arranger::default()
        };
        let first = arranger.add_section("A", 0.0, 4.0).unwrap();
        let second = arranger.add_section("B", 4.0, 8.0).unwrap();
        assert!(arranger.append_sequence(first, 1));
        assert!(arranger.append_sequence(second, 1));
        assert!(arranger.remove_section(first));
        assert_eq!(arranger.sequence.len(), 1);
        assert_eq!(arranger.sequence[0].section_id, second);

        assert!(arranger.add_section("", 0.0, 1.0).is_none());
        assert!(arranger.add_section("Inverted", 4.0, 4.0).is_none());
        assert!(!arranger.set_sequence_repeats(0, 0));
        assert!(!arranger.append_sequence(999, 1));
        assert_eq!(arranger.validate(), Ok(()));
    }

    #[test]
    fn bounds_and_reference_validation_are_enforced() {
        let mut arranger = Arranger {
            next_section_id: 1,
            ..Arranger::default()
        };
        let section = arranger.add_section("Loop", 0.0, 8.0).unwrap();
        assert!(arranger.append_sequence(section, 1024));
        assert!(!arranger.append_sequence(section, 1));
        assert_eq!(arranger.validate(), Ok(()));
        arranger.sequence.push(ArrangerSequenceItem {
            section_id: 999,
            repeats: 1,
        });
        assert!(arranger.validate().is_err());
    }
}
