"""Generate the multi-size Windows ICO from opentrack-icon.svg geometry."""

from pathlib import Path

from PIL import Image, ImageDraw


OUTPUT = Path(__file__).resolve().parents[1] / "crates/opentrack-app/assets/opentrack-icon.ico"
MASTER = 2048
SCALE = MASTER / 28


def pixel(value: float) -> int:
    return round((value - 2) * SCALE)


image = Image.new("RGBA", (MASTER, MASTER), (0, 0, 0, 0))
draw = ImageDraw.Draw(image)
white = (255, 255, 255, 255)
draw.ellipse(
    (pixel(4), pixel(4), pixel(28), pixel(28)),
    outline=white,
    width=round(2 * SCALE),
)
for cx, cy, radius in ((16, 16, 3), (16, 8.5, 2), (22.495, 19.75, 2), (9.505, 19.75, 2)):
    draw.ellipse(
        (pixel(cx - radius), pixel(cy - radius), pixel(cx + radius), pixel(cy + radius)),
        fill=white,
    )

OUTPUT.parent.mkdir(parents=True, exist_ok=True)
image.save(
    OUTPUT,
    format="ICO",
    sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
)
print(OUTPUT)
