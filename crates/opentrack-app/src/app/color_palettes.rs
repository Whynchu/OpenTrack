//! Built-in palettes for the project's 16 event-color slots.
//!
//! A palette is a complete set of 16 colors, not an offset or a filter: choosing
//! one replaces every slot, so the arrange always has a usable full palette and
//! the result is predictable. The colors themselves are stored in the workspace
//! view state, so no palette identity needs to be persisted and the project
//! format does not change; the dialog recovers the current palette by comparing
//! the stored slots.

use crate::project_file::DEFAULT_EVENT_COLORS;

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
            [150, 121, 104],
            [162, 132, 108],
            [173, 143, 112],
            [158, 147, 109],
            [141, 150, 112],
            [124, 143, 112],
            [112, 136, 116],
            [106, 130, 124],
            [110, 128, 134],
            [113, 124, 139],
            [120, 120, 142],
            [130, 116, 141],
            [140, 114, 137],
            [148, 118, 130],
            [155, 122, 122],
            [160, 126, 114],
        ],
    },
    Palette {
        name: "Cool Studio",
        colors: [
            [92, 141, 176],
            [80, 152, 168],
            [76, 160, 152],
            [82, 156, 132],
            [96, 150, 120],
            [104, 142, 132],
            [96, 134, 148],
            [90, 126, 160],
            [92, 116, 170],
            [104, 108, 172],
            [116, 100, 164],
            [130, 96, 152],
            [142, 96, 140],
            [150, 100, 128],
            [156, 106, 118],
            [158, 118, 116],
        ],
    },
    Palette {
        name: "High Contrast",
        colors: [
            [230, 80, 60],
            [240, 140, 50],
            [245, 200, 40],
            [170, 220, 50],
            [90, 210, 90],
            [40, 200, 140],
            [40, 200, 200],
            [50, 170, 235],
            [70, 120, 245],
            [110, 90, 240],
            [150, 70, 230],
            [200, 70, 200],
            [235, 70, 150],
            [240, 70, 100],
            [230, 90, 70],
            [200, 110, 60],
        ],
    },
    Palette {
        name: "Pastel",
        colors: [
            [240, 190, 180],
            [245, 210, 180],
            [245, 228, 180],
            [232, 238, 186],
            [204, 232, 196],
            [190, 226, 208],
            [186, 222, 222],
            [190, 214, 236],
            [196, 204, 240],
            [206, 198, 238],
            [218, 196, 232],
            [230, 196, 226],
            [240, 198, 214],
            [242, 200, 200],
            [238, 206, 196],
            [232, 214, 200],
        ],
    },
    Palette {
        name: "Grayscale",
        colors: [
            [235, 235, 235],
            [220, 220, 220],
            [205, 205, 205],
            [190, 190, 190],
            [175, 175, 175],
            [160, 160, 160],
            [145, 145, 145],
            [130, 130, 130],
            [118, 118, 118],
            [106, 106, 106],
            [94, 94, 94],
            [82, 82, 82],
            [70, 70, 70],
            [58, 58, 58],
            [46, 46, 46],
            [36, 36, 36],
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
