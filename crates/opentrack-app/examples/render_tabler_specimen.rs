use std::fs;

fn main() {
    let icons = [
        ("Play", "player-play.svg", true),
        ("Stop", "player-stop.svg", true),
        ("Record", "player-record.svg", true),
        ("Select", "pointer.svg", false),
        ("Range", "select.svg", false),
        ("Split", "scissors.svg", false),
        ("Draw", "pencil.svg", false),
        ("Erase", "eraser.svg", false),
        ("Zoom", "zoom-in.svg", false),
        ("Mute", "volume-off.svg", false),
        ("Snap", "magnet.svg", false),
        ("Mixer", "adjustments-horizontal.svg", false),
        ("Monitor", "volume.svg", false),
        ("Phase", "circle-off.svg", false),
        ("Insert", "plug.svg", false),
        ("EQ", "chart-line.svg", false),
        ("Routing", "route.svg", false),
        ("Track Audio", "wave-sine.svg", false),
        ("Track MIDI", "piano.svg", false),
        ("Instrument", "keyboard.svg", false),
        ("Power", "power.svg", false),
        ("Undo", "arrow-back-up.svg", false),
    ];
    let states = [
        ("Default", "#a0a0a0", "#151719"),
        ("Hover", "#b2b2b2", "#101214"),
        ("Active", "#347fbd", "#ffffff"),
        ("Disabled", "#a0a0a0", "#64676b"),
    ];
    let width = 1080;
    let height = 1120;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#1d1f22\"/>\
         <style>text{{font-family:Segoe UI,Arial,sans-serif;fill:#e2e5e9}} .muted{{fill:#9299a2}}</style>\
         <text x=\"28\" y=\"38\" font-size=\"22\" font-weight=\"600\">OpenTrack · Tabler toolbar specimen</text>\
         <text x=\"28\" y=\"60\" font-size=\"12\" class=\"muted\">v3.49.0 · outline 1.75-unit strokes · familiar filled transport silhouettes · enlarged states</text>"
    );
    let first_x = 250;
    let col_w = 195;
    for (column, (label, _, _)) in states.iter().enumerate() {
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"96\" text-anchor=\"middle\" font-size=\"14\" font-weight=\"600\">{label}</text>",
            first_x + column * col_w + 54
        ));
    }
    for (row, (label, file, filled)) in icons.iter().enumerate() {
        let y = 117 + row * 45;
        svg.push_str(&format!(
            "<text x=\"28\" y=\"{}\" font-size=\"14\">{label}</text>",
            y + 23
        ));
        let asset = fs::read_to_string(format!("crates/opentrack-app/assets/tabler/{file}"))
            .expect("vendored Tabler SVG");
        let start = asset.find("<svg").expect("SVG root");
        let content_start = asset[start..].find('>').expect("SVG root end") + start + 1;
        let content_end = asset.rfind("</svg>").expect("SVG root close");
        let contents = &asset[content_start..content_end];
        for (column, (_, background, icon_color)) in states.iter().enumerate() {
            let x = first_x + column * col_w + 36;
            let color = if label == &"Record" && column < 2 {
                "#dc4141"
            } else {
                icon_color
            };
            let (fill, stroke, stroke_width) = if *filled {
                (color, "none", "0")
            } else {
                ("none", color, "1.75")
            };
            svg.push_str(&format!(
                "<rect x=\"{x}\" y=\"{y}\" width=\"36\" height=\"36\" rx=\"4\" fill=\"{background}\" stroke=\"#4b4f55\" stroke-opacity=\".35\"/>\
                 <svg x=\"{}\" y=\"{}\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{contents}</svg>",
                x + 6,
                y + 6
            ));
        }
    }
    svg.push_str(
        "<text x=\"28\" y=\"1092\" font-size=\"11\" class=\"muted\">Button sizes shown enlarged for inspection; application hit areas and spacing remain unchanged. Transport preserves red record identity and existing accents.</text></svg>",
    );
    fs::write("docs/reviews/tabler-toolbar-specimen.svg", svg).expect("write specimen sheet");
}
