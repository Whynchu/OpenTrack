//! MIDI editor preference defaults, persistence, and quantization policy.

pub(crate) const MIDI_QUANTIZE_VALUES: [(f64, &str); 7] = [
    (0.0625, "1/64 note"),
    (0.125, "1/32 note"),
    (0.25, "1/16 note"),
    (0.5, "1/8 note"),
    (1.0, "1/4 note"),
    (2.0, "1/2 note"),
    (4.0, "Whole note"),
];

pub(crate) struct MidiEditorSettings {
    pub(crate) snap_enabled: bool,
    pub(crate) position_grid_beats: f64,
    pub(crate) length_grid_beats: f64,
    pub(crate) insert_velocity: u8,
}

impl MidiEditorSettings {
    pub(crate) fn load(storage: Option<&dyn eframe::Storage>) -> Self {
        Self {
            snap_enabled: storage
                .and_then(|storage| eframe::get_value(storage, "midi-editor-snap-enabled"))
                .unwrap_or(true),
            position_grid_beats: storage
                .and_then(|storage| eframe::get_value(storage, "midi-position-quantize"))
                .filter(|value| is_quantize_value(*value))
                .unwrap_or(1.0),
            length_grid_beats: storage
                .and_then(|storage| eframe::get_value(storage, "midi-length-quantize"))
                .filter(|value| is_quantize_value(*value))
                .unwrap_or(0.25),
            insert_velocity: storage
                .and_then(|storage| eframe::get_value(storage, "midi-insert-velocity"))
                .filter(|value| (1..=127).contains(value))
                .unwrap_or(96),
        }
    }

    pub(crate) fn save(&self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "midi-editor-snap-enabled", &self.snap_enabled);
        eframe::set_value(storage, "midi-position-quantize", &self.position_grid_beats);
        eframe::set_value(storage, "midi-length-quantize", &self.length_grid_beats);
        eframe::set_value(storage, "midi-insert-velocity", &self.insert_velocity);
    }
}

pub(crate) fn is_quantize_value(value: f64) -> bool {
    MIDI_QUANTIZE_VALUES
        .iter()
        .any(|(candidate, _)| value == *candidate)
}

pub(crate) fn position_quantize(grid: f64, snap_enabled: bool) -> f64 {
    let grid = if grid.is_finite() && grid > 0.0 {
        grid
    } else {
        1.0
    };
    if snap_enabled {
        grid
    } else {
        // Preserve practical beat precision while preventing the selected grid
        // from affecting note and controller positions when Snap is off.
        1.0e-9
    }
}

#[cfg(test)]
mod tests {
    use super::{is_quantize_value, position_quantize};

    #[test]
    fn snap_enabled_uses_the_selected_position_grid() {
        assert_eq!(position_quantize(0.125, true), 0.125);
    }

    #[test]
    fn snap_disabled_preserves_fine_position_precision() {
        let quantize = position_quantize(0.25, false);
        let beat = 0.312_345_678_9;
        let rounded = (beat / quantize).round() * quantize;
        assert!((rounded - beat).abs() <= quantize);
        assert_ne!(rounded, (beat / 0.25).round() * 0.25);
    }

    #[test]
    fn unsupported_grids_are_rejected() {
        assert!(!is_quantize_value(0.3));
    }
}
