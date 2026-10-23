#!/usr/bin/env python3
"""Tile the frames of a `HZ_SCENE=play` run into one film strip for review.

    python3 -I scripts/strip.py docs/play docs/play_strip.png [columns]

Reads docs/play_00.png, docs/play_01.png, ... and writes a downscaled grid with the
frame index stamped in each corner. Needs Pillow.
"""
import glob
import sys

from PIL import Image, ImageDraw

prefix = sys.argv[1]
out = sys.argv[2]
cols = int(sys.argv[3]) if len(sys.argv) > 3 else 2
paths = sorted(glob.glob(f"{prefix}_[0-9][0-9].png"))
if not paths:
    sys.exit(f"no frames matching {prefix}_NN.png")

frames = [Image.open(p).convert("RGB") for p in paths]
w, h = frames[0].size
scale = 0.5
tw, th = int(w * scale), int(h * scale)
rows = (len(frames) + cols - 1) // cols
sheet = Image.new("RGB", (cols * tw, rows * th), (0, 0, 0))
draw = ImageDraw.Draw(sheet)
for i, f in enumerate(frames):
    x, y = (i % cols) * tw, (i // cols) * th
    sheet.paste(f.resize((tw, th), Image.NEAREST), (x, y))
    draw.rectangle([x, y, x + 34, y + 16], fill=(0, 0, 0))
    draw.text((x + 4, y + 2), f"#{i:02d}", fill=(232, 168, 56))
sheet.save(out)
print(f"{len(frames)} frames -> {out}")
