use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GridType {
    #[default]
    Bar,
    Beat,
    UseQuantize,
}

impl GridType {
    pub(crate) const ALL: [Self; 3] = [Self::Bar, Self::Beat, Self::UseQuantize];
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Bar => "Bar",
            Self::Beat => "Beat",
            Self::UseQuantize => "Use Quantize",
        }
    }
    pub(crate) fn step_beats(self, beats_per_bar: u8, quantize: QuantizeSetting) -> f64 {
        match self {
            Self::Bar => f64::from(beats_per_bar.max(1)),
            Self::Beat => 1.0,
            Self::UseQuantize => quantize.step_beats(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QuantizeNote {
    Whole,
    Half,
    Quarter,
    Eighth,
    #[default]
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
    OneTwentyEighth,
}

impl QuantizeNote {
    pub(crate) const ALL: [Self; 8] = [
        Self::Whole,
        Self::Half,
        Self::Quarter,
        Self::Eighth,
        Self::Sixteenth,
        Self::ThirtySecond,
        Self::SixtyFourth,
        Self::OneTwentyEighth,
    ];
    fn denominator(self) -> u16 {
        match self {
            Self::Whole => 1,
            Self::Half => 2,
            Self::Quarter => 4,
            Self::Eighth => 8,
            Self::Sixteenth => 16,
            Self::ThirtySecond => 32,
            Self::SixtyFourth => 64,
            Self::OneTwentyEighth => 128,
        }
    }
    pub(crate) fn label(self) -> String {
        format!("1/{}", self.denominator())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QuantizeVariation {
    #[default]
    Straight,
    Triplet,
    Dotted,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct QuantizeSetting {
    pub(crate) note: QuantizeNote,
    pub(crate) variation: QuantizeVariation,
}

impl QuantizeSetting {
    pub(crate) fn step_beats(self) -> f64 {
        4.0 / f64::from(self.note.denominator())
            * match self.variation {
                QuantizeVariation::Straight => 1.0,
                QuantizeVariation::Triplet => 2.0 / 3.0,
                QuantizeVariation::Dotted => 1.5,
            }
    }
    pub(crate) fn label(self) -> String {
        format!(
            "{}{}",
            self.note.label(),
            match self.variation {
                QuantizeVariation::Straight => "",
                QuantizeVariation::Triplet => " T",
                QuantizeVariation::Dotted => ".",
            }
        )
    }
}

/// Snap a non-negative project beat position to the nearest enabled grid line.
/// Project transport positions are quarter-note beats. The toolbar's Bar grid
/// therefore uses the project's beats-per-bar, independently of MIDI quantize.
pub(crate) fn snap_beats_to_grid(position_beats: f64, grid_beats: f64) -> f64 {
    if !position_beats.is_finite() {
        return 0.0;
    }

    let position_beats = position_beats.max(0.0);
    if !grid_beats.is_finite() || grid_beats <= 0.0 {
        return position_beats;
    }

    (position_beats / grid_beats).round() * grid_beats
}

pub(crate) fn ruler_position_beats(
    pointer_x: f32,
    grid_origin_x: f32,
    bar_width: f32,
    beats_per_bar: u8,
) -> f64 {
    f64::from((pointer_x - grid_origin_x).max(0.0) / bar_width.max(1.0))
        * f64::from(beats_per_bar.max(1))
}

pub(crate) fn edge_scroll_offset(
    pointer_x: f32,
    left: f32,
    right: f32,
    current: f32,
    maximum: f32,
    frame_dt: f32,
) -> f32 {
    const ZONE: f32 = 32.0;
    let left_strength = ((left + ZONE - pointer_x) / ZONE).clamp(0.0, 1.0);
    let right_strength = ((pointer_x - (right - ZONE)) / ZONE).clamp(0.0, 1.0);
    let direction = right_strength.powi(2) - left_strength.powi(2);
    (current + direction * 720.0 * frame_dt.clamp(0.001, 0.05)).clamp(0.0, maximum.max(0.0))
}

pub(crate) fn cycle_color(mode: opentrack_core::project::CycleMode) -> egui::Color32 {
    match mode {
        opentrack_core::project::CycleMode::Loop => egui::Color32::from_rgb(151, 94, 202),
        opentrack_core::project::CycleMode::Skip => egui::Color32::from_rgb(218, 91, 52),
    }
}

/// Snap the translation, rather than each endpoint, so fractional range lengths
/// and the grab offset survive movement. Clamp both ends as one rigid interval.
pub(crate) fn translate_range(
    anchor: f64,
    endpoint: f64,
    delta: f64,
    grid: Option<f64>,
) -> (f64, f64) {
    let delta = if let Some(grid) = grid.filter(|grid| grid.is_finite() && *grid > 0.0) {
        (delta / grid).round() * grid
    } else {
        delta
    };
    let delta = delta.clamp(-anchor.min(endpoint), 1_000_000.0 - anchor.max(endpoint));
    (anchor + delta, endpoint + delta)
}

pub(crate) fn range_edge_at_x(pointer_x: f32, left: f32, right: f32) -> Option<bool> {
    let left_distance = (pointer_x - left).abs();
    let right_distance = (pointer_x - right).abs();
    let handle_size = ((right - left).max(0.0) * 0.25).clamp(0.5, 7.0);
    if left_distance.min(right_distance) <= handle_size {
        Some(left_distance <= right_distance)
    } else {
        None
    }
}

/// Resize one chronological boundary without crossing the fixed boundary or
/// changing Loop/Skip direction. A snapped edge stops at the last valid grid
/// line before the opposite edge; fractional fixed edges stay untouched.
pub(crate) fn resize_range(
    anchor: f64,
    endpoint: f64,
    left: bool,
    position: f64,
    grid: Option<f64>,
) -> (f64, f64) {
    let grid = grid.filter(|grid| grid.is_finite() && *grid > 0.0);
    let position = grid.map_or(position, |grid| snap_beats_to_grid(position, grid));
    let (mut low, mut high) = (anchor.min(endpoint), anchor.max(endpoint));
    if left {
        let limit = grid.map_or(high - 0.000_001, |grid| ((high / grid).ceil() - 1.0) * grid);
        low = position.clamp(0.0, limit.max(0.0));
    } else {
        let limit = grid.map_or(low + 0.000_001, |grid| ((low / grid).floor() + 1.0) * grid);
        high = position.clamp(limit.min(1_000_000.0), 1_000_000.0);
    }
    if anchor < endpoint {
        (low, high)
    } else {
        (high, low)
    }
}

pub(crate) const MIN_ZOOM: f32 = 0.25;
pub(crate) const MAX_ZOOM: f32 = 256.0;

pub(crate) fn move_cursor_icon(held: bool) -> egui::CursorIcon {
    if held {
        egui::CursorIcon::Move
    } else {
        egui::CursorIcon::PointingHand
    }
}

pub(crate) fn show_move_cursor(ui: &egui::Ui, held: bool) {
    ui.ctx().set_cursor_icon(move_cursor_icon(held));
}

/// Finest quarter/eighth/sixteenth division that leaves the requested space.
/// Zero means bar-level rendering only; result units are divisions per quarter.
pub(crate) fn ruler_subdivisions(bar_width: f32, beats_per_bar: u8, minimum_spacing: f32) -> usize {
    let quarter_width = bar_width / f32::from(beats_per_bar.max(1));
    [1, 2, 4]
        .into_iter()
        .filter(|division| quarter_width / (*division as f32) >= minimum_spacing)
        .last()
        .unwrap_or(0)
}

/// Compact Bars+Beats labels. Sub-quarter labels use sixteenth positions so
/// eighths remain 1/3 and sixteenths 1/2/3/4 as the ruler refines.
pub(crate) fn subdivision_label(bar: usize, slot: usize, divisions: usize) -> String {
    let beat = slot / divisions + 1;
    let remainder = slot % divisions;
    if remainder == 0 {
        format!("{}.{}", bar + 1, beat)
    } else {
        format!("{}.{}.{}", bar + 1, beat, remainder * 4 / divisions + 1)
    }
}

/// Stable, globally anchored 1/2/4/8... bar intervals. Label width is measured
/// for the current format and visible bar numbers, independently of snap.
pub(crate) fn ruler_stride(bar_width: f32, minimum_spacing: f32) -> usize {
    let mut stride = 1_usize;
    while bar_width.max(1.0) * (stride as f32) < minimum_spacing.max(1.0) && stride < 1 << 20 {
        stride *= 2;
    }
    stride
}

#[cfg(test)]
mod tests {
    use super::snap_beats_to_grid;

    #[test]
    fn snaps_to_nearest_sixteenth_note_in_quarter_note_beats() {
        assert_eq!(snap_beats_to_grid(1.13, 0.25), 1.25);
        assert_eq!(snap_beats_to_grid(1.12, 0.25), 1.0);
        assert_eq!(snap_beats_to_grid(1.0, 0.25), 1.0);
    }

    #[test]
    fn positive_half_grid_tie_rounds_forward() {
        assert_eq!(snap_beats_to_grid(1.125, 0.25), 1.25);
    }

    #[test]
    fn bar_grid_is_independent_of_sixteenth_note_quantize() {
        assert_eq!(snap_beats_to_grid(65.1, 4.0), 64.0);
        assert_eq!(snap_beats_to_grid(66.1, 4.0), 68.0);
        assert_eq!(snap_beats_to_grid(8.6, 3.0), 9.0);
    }

    #[test]
    fn grid_type_resolves_bar_beat_and_straight_triplet_dotted_quantize() {
        use super::*;
        let mut q = QuantizeSetting {
            note: QuantizeNote::Eighth,
            variation: QuantizeVariation::Straight,
        };
        assert_eq!(GridType::Bar.step_beats(3, q), 3.0);
        assert_eq!(GridType::Beat.step_beats(3, q), 1.0);
        assert_eq!(GridType::UseQuantize.step_beats(3, q), 0.5);
        q.variation = QuantizeVariation::Triplet;
        assert!((GridType::UseQuantize.step_beats(4, q) - 1.0 / 3.0).abs() < 1e-12);
        assert_eq!(q.label(), "1/8 T");
        q.variation = QuantizeVariation::Dotted;
        assert_eq!(GridType::UseQuantize.step_beats(4, q), 0.75);
        assert_eq!(q.label(), "1/8.");
    }

    #[test]
    fn ruler_density_increases_with_zoom_and_leaves_room_for_wide_labels() {
        assert_eq!(super::ruler_stride(80.0, 32.0), 1);
        assert_eq!(super::ruler_stride(20.0, 32.0), 2);
        assert_eq!(super::ruler_stride(10.0, 32.0), 4);
        assert_eq!(super::ruler_stride(4.0, 32.0), 8);
        assert_eq!(super::ruler_stride(20.0, 70.0), 4);
    }

    #[test]
    fn zoom_refines_bars_into_quarters_eighths_and_sixteenths_then_reverses() {
        let resolutions = [24.0, 48.0, 96.0, 192.0, 96.0, 48.0, 24.0]
            .map(|width| super::ruler_subdivisions(width, 4, 12.0));
        assert_eq!(resolutions, [0, 1, 2, 4, 2, 1, 0]);
        assert_eq!(super::ruler_subdivisions(144.0, 3, 12.0), 4);
        assert_eq!(super::ruler_subdivisions(192.0, 4, 50.0), 0);
        assert_eq!(super::ruler_subdivisions(800.0, 4, 50.0), 4);
        assert_eq!(super::subdivision_label(4, 1, 1), "5.2");
        assert_eq!(super::subdivision_label(4, 1, 2), "5.1.3");
        assert_eq!(super::subdivision_label(4, 1, 4), "5.1.2");
        assert_eq!(super::subdivision_label(4, 4, 4), "5.2");
    }

    #[test]
    fn range_translation_preserves_length_direction_and_project_limits() {
        assert_eq!(
            super::translate_range(8.5, 17.25, 5.0, Some(4.0)),
            (12.5, 21.25)
        );
        assert_eq!(
            super::translate_range(17.25, 8.5, 5.0, Some(4.0)),
            (21.25, 12.5)
        );
        assert_eq!(super::translate_range(8.5, 17.25, -50.0, None), (0.0, 8.75));
        assert_eq!(
            super::translate_range(999_990.0, 999_999.0, 10.0, None),
            (999_991.0, 1_000_000.0)
        );
    }

    #[test]
    fn range_edges_resize_independently_preserving_direction_and_validity() {
        assert_eq!(
            super::resize_range(8.0, 24.0, true, 3.0, Some(4.0)),
            (4.0, 24.0)
        );
        assert_eq!(
            super::resize_range(24.0, 8.0, false, 33.0, Some(4.0)),
            (32.0, 8.0)
        );
        assert_eq!(
            super::resize_range(8.0, 24.0, true, 50.0, Some(4.0)),
            (20.0, 24.0)
        );
        assert_eq!(
            super::resize_range(8.0, 24.0, false, 0.0, Some(4.0)),
            (8.0, 12.0)
        );
        assert_eq!(
            super::resize_range(8.5, 24.25, true, -5.0, None),
            (0.0, 24.25)
        );
        assert_eq!(
            super::resize_range(8.5, 24.25, false, 1_000_010.0, None),
            (8.5, 1_000_000.0)
        );
    }

    #[test]
    fn pointer_mapping_accounts_for_scroll_and_clamps_project_start() {
        assert_eq!(super::ruler_position_beats(430.0, 350.0, 40.0, 4), 8.0);
        assert_eq!(super::ruler_position_beats(300.0, 350.0, 40.0, 4), 0.0);
    }

    #[test]
    fn held_edge_scroll_advances_both_directions_and_stops_in_the_middle() {
        let right = super::edge_scroll_offset(500.0, 100.0, 500.0, 100.0, 1000.0, 0.02);
        assert!(right > 100.0);
        assert!(super::edge_scroll_offset(500.0, 100.0, 500.0, right, 1000.0, 0.02) > right);
        assert!(super::edge_scroll_offset(100.0, 100.0, 500.0, 100.0, 1000.0, 0.02) < 100.0);
        assert_eq!(
            super::edge_scroll_offset(300.0, 100.0, 500.0, 100.0, 1000.0, 0.02),
            100.0
        );
        assert_eq!(
            super::edge_scroll_offset(100.0, 100.0, 500.0, 0.0, 1000.0, 0.02),
            0.0
        );
    }

    #[test]
    fn clamps_negative_transport_positions_to_project_start() {
        assert_eq!(snap_beats_to_grid(-0.2, 0.25), 0.0);
    }

    #[test]
    fn invalid_grid_leaves_finite_position_unsnapped() {
        assert_eq!(snap_beats_to_grid(1.13, 0.0), 1.13);
        assert_eq!(snap_beats_to_grid(1.13, f64::NAN), 1.13);
    }

    #[test]
    fn non_finite_transport_position_returns_to_project_start() {
        assert_eq!(snap_beats_to_grid(f64::NAN, 0.25), 0.0);
        assert_eq!(snap_beats_to_grid(f64::INFINITY, 0.25), 0.0);
    }
}
