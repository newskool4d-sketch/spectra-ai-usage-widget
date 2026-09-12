"""Rasterize checked-in provider SVGs to 64x64 alpha masks (development only).

Requires PyMuPDF. The Rust app embeds the masks; its build/runtime needs no SVG
renderer, image decoder, network access, or additional Cargo dependency.
"""

from pathlib import Path
import xml.etree.ElementTree as ET

import fitz


SIDE = 64
ASSETS = Path(__file__).resolve().parents[1] / "src-tauri" / "assets" / "provider-ci"


def main() -> None:
    ET.register_namespace("", "http://www.w3.org/2000/svg")
    for name in ("codex", "claude"):
        root = ET.fromstring((ASSETS / f"{name}.svg").read_bytes())
        root.set("width", str(SIDE))
        root.set("height", str(SIDE))
        root.set("fill", "#000000")
        with fitz.open(stream=ET.tostring(root), filetype="svg") as svg:
            pdf_bytes = svg.convert_to_pdf()
        with fitz.open(stream=pdf_bytes, filetype="pdf") as pdf:
            page = pdf[0]
            pixels = page.get_pixmap(
                matrix=fitz.Matrix(SIDE / page.rect.width, SIDE / page.rect.height),
                alpha=True,
            )
        assert (pixels.width, pixels.height, pixels.n) == (SIDE, SIDE, 4)
        mask = bytes(pixels.samples[3::4])
        assert len(mask) == SIDE * SIDE and min(mask) == 0 and max(mask) == 255
        (ASSETS / f"{name}.alpha").write_bytes(mask)
        print(f"{name}: {SIDE}x{SIDE}, {len(mask)} bytes")


if __name__ == "__main__":
    main()
