//! Built-in palettes for the project's 16 event-color slots.
//!
//! A palette is a complete set of 16 colors, not an offset or a filter: choosing
//! one replaces every slot, so the arrange always has a usable full palette and
//! the result is predictable. The colors themselves are stored in the workspace
//! view state, so no palette identity needs to be persisted and the project
//! format does not change; the dialog recovers the current palette by comparing
//! the stored slots.

use crate::project_file::{DEFAULT_EVENT_COLOR_GREY, DEFAULT_EVENT_COLORS};

/// One selectable set of 16 event colors.
pub(super) struct Palette {
    pub name: &'static str,
    pub colors: [[u8; 3]; 16],
}

/// The palettes offered by Event Colors, in menu order.
///
/// The first entry is the set OpenTrack has always shipped, so "Default" means
/// the colors an existing project already shows.
pub(super) const PALETTES: [Palette; 6] = [
    Palette {
        name: "Default",
        colors: DEFAULT_EVENT_COLORS,
    },
    Palette {
        name: "Muted Earth",
        colors: [
            DEFAULT_EVENT_COLOR_GREY,
            [166, 106, 54],
            [174, 134, 50],
            [160, 152, 52],
            [128, 148, 54],
            [96, 138, 64],
            [70, 128, 88],
            [58, 118, 104],
            [60, 108, 116],
            [66, 96, 120],
            [78, 84, 118],
            [96, 76, 112],
            [116, 70, 102],
            [134, 74, 92],
            [146, 82, 78],
            [148, 94, 64],
        ],
    },
    Palette {
        name: "Cool Studio",
        colors: [
            DEFAULT_EVENT_COLOR_GREY,
            [38, 136, 178],
            [34, 150, 160],
            [36, 152, 130],
            [48, 146, 104],
            [48, 136, 132],
            [44, 122, 152],
            [40, 106, 166],
            [44, 88, 176],
            [64, 76, 172],
            [88, 68, 158],
            [112, 64, 142],
            [134, 64, 124],
            [150, 70, 106],
            [156, 80, 92],
            [152, 92, 84],
        ],
    },
    Palette {
        name: "High Contrast",
        colors: [
            DEFAULT_EVENT_COLOR_GREY,
            [246, 128, 24],
            [248, 198, 16],
            [164, 226, 24],
            [60, 214, 56],
            [16, 206, 124],
            [12, 202, 202],
            [22, 158, 244],
            [44, 98, 252],
            [92, 60, 246],
            [146, 42, 232],
            [206, 42, 200],
            [240, 44, 140],
            [246, 44, 76],
            [228, 66, 44],
            [196, 100, 36],
        ],
    },
    Palette {
        name: "Pastel",
        colors: [
            DEFAULT_EVENT_COLOR_GREY,
            [252, 202, 148],
            [252, 228, 146],
            [222, 240, 148],
            [176, 234, 164],
            [152, 228, 188],
            [148, 222, 222],
            [154, 206, 242],
            [162, 190, 248],
            [182, 174, 246],
            [208, 166, 238],
            [232, 164, 230],
            [250, 166, 204],
            [252, 170, 170],
            [244, 186, 160],
            [232, 198, 168],
        ],
    },
    Palette {
        name: "Grayscale",
        colors: [
            DEFAULT_EVENT_COLOR_GREY,
            [226, 226, 226],
            [210, 210, 210],
            [194, 194, 194],
            [178, 178, 178],
            [162, 162, 162],
            [146, 146, 146],
            [130, 130, 130],
            [116, 116, 116],
            [102, 102, 102],
            [88, 88, 88],
            [74, 74, 74],
            [60, 60, 60],
            [48, 48, 48],
            [38, 38, 38],
            [28, 28, 28],
        ],
    },
];

/// The palette whose colors exactly match `colors`, if any.
pub(super) fn matching_palette(colors: &[[u8; 3]; 16]) -> Option<usize> {
    PALETTES
        .iter()
        .position(|palette| &palette.colors == colors)
}

/// The palette's name, or "Custom" for a hand-edited set.
pub(super) fn palette_name(index: Option<usize>) -> &'static str {
    index
        .and_then(|index| PALETTES.get(index))
        .map_or("Custom", |palette| palette.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_palette_is_a_complete_distinct_set_and_the_first_is_the_default() {
        assert!(
            PALETTES.len() >= 5,
            "Event Colors offers at least five palettes"
        );
        assert_eq!(PALETTES[0].colors, DEFAULT_EVENT_COLORS);
        assert_eq!(PALETTES[0].name, "Default");

        for (index, palette) in PALETTES.iter().enumerate() {
            assert!(!palette.name.trim().is_empty());
            assert_eq!(
                palette.colors[0], DEFAULT_EVENT_COLOR_GREY,
                "{} starts with the standard grey",
                palette.name
            );
            // Every palette is a full 16-slot set with opaque, usable colors.
            assert_eq!(palette.colors.len(), 16, "{}", palette.name);
            assert!(
                palette
                    .colors
                    .iter()
                    .all(|color| color.iter().any(|channel| *channel > 0)),
                "{} must not contain pure black slots",
                palette.name
            );
            for (other_index, other) in PALETTES.iter().enumerate().skip(index + 1) {
                assert_ne!(
                    palette.colors, other.colors,
                    "{} and {} are the same palette",
                    palette.name, other.name
                );
                assert_ne!(palette.name, other.name);
            }
            assert_eq!(matching_palette(&palette.colors), Some(index));
        }

        // A hand-edited set is not mistaken for a built-in palette.
        let mut custom = PALETTES[0].colors;
        custom[7] = [1, 2, 3];
        assert_eq!(matching_palette(&custom), None);
        assert_eq!(palette_name(None), "Custom");
        assert_eq!(palette_name(Some(0)), "Default");
        assert_eq!(palette_name(Some(usize::MAX)), "Custom");
    }
}
