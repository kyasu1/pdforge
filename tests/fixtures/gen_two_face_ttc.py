#!/usr/bin/env python3
"""Generate tests/fixtures/two-face-test.ttc: a minimal 2-face TrueType
Collection (TTC) used to test PDForgeBuilder's font_index parameter.

Requires fontTools (pip install fonttools). Run from anywhere:

    python3 tests/fixtures/gen_two_face_ttc.py

Face 0 ("TestFaceA") has 1 glyph beyond .notdef; face 1 ("TestFaceB") has
2, so tests can assert that selecting font_index=1 actually loads a
different face (distinguishable by glyph count) rather than silently
reusing index 0.
"""

import os

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTCollection, TTFont

OUT_DIR = os.path.dirname(os.path.abspath(__file__))


def _rect_glyph():
    pen = TTGlyphPen(None)
    pen.moveTo((0, 0))
    pen.lineTo((0, 700))
    pen.lineTo((500, 700))
    pen.lineTo((500, 0))
    pen.closePath()
    return pen.glyph()


def make_font(family, extra_glyphs):
    """Build a minimal valid TTF with .notdef plus `extra_glyphs` (a dict of
    glyph_name -> unicode codepoint)."""
    fb = FontBuilder(1000, isTTF=True)
    glyph_order = [".notdef"] + list(extra_glyphs.keys())
    fb.setupGlyphOrder(glyph_order)
    fb.setupCharacterMap(
        {codepoint: name for name, codepoint in extra_glyphs.items()}
    )

    notdef_pen = TTGlyphPen(None)
    glyphs = {".notdef": notdef_pen.glyph()}
    for name in extra_glyphs:
        glyphs[name] = _rect_glyph()
    fb.setupGlyf(glyphs)

    metrics = {name: (500, 0) for name in glyph_order}
    fb.setupHorizontalMetrics(metrics)
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName": family, "styleName": "Regular"})
    fb.setupOS2()
    fb.setupPost()
    return fb.font


def main():
    face0 = make_font("TestFaceA", {"A": ord("A")})
    face1 = make_font("TestFaceB", {"A": ord("A"), "B": ord("B")})

    ttc = TTCollection()
    ttc.fonts = [face0, face1]

    out_path = os.path.join(OUT_DIR, "two-face-test.ttc")
    ttc.save(out_path)
    print(f"wrote {out_path} ({os.path.getsize(out_path)} bytes)")

    # Sanity check: reload and confirm both faces are distinguishable.
    reloaded = TTCollection(out_path)
    for i, font in enumerate(reloaded.fonts):
        name = font["name"].getDebugName(1)
        num_glyphs = font["maxp"].numGlyphs
        print(f"  face {i}: name={name!r} num_glyphs={num_glyphs}")


if __name__ == "__main__":
    main()
