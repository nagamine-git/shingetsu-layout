#!/usr/bin/env python3
"""Generate a keyboard layout SVG image from shingetsu_analyzer.json."""

import json
import os

# ---------------------------------------------------------------------------
# Dimensions
# ---------------------------------------------------------------------------
U   = 57          # 1u unit (key + gap)
KW  = 53          # key width for 1u
KH  = 53          # key height
GAP = U - KW      # 4 px gap between keys
PAD = 16          # outer padding
TTL = 34          # title area height
R   = 6           # corner radius of key rect

# Row stagger (ANSI)
STAGGER = [0, 0.25 * U, 0.5 * U]   # for rows index 1, 2, 3 → mapped to 0, 1, 2

# Modifier key IDs to skip (just advance x)
MODIFIERS = {"tab", "caps", "shift", "shiftr", "enter", "backslash", "\\"}

# Finger → pastel fill colour (lighter/paler)
FINGER_COLORS = {
    0: "#fce8f0",   # pink (lighter)
    1: "#d8f0ed",   # teal (lighter)
    2: "#e0f5e0",   # green (lighter)
    3: "#fff5d0",   # yellow (lighter)
    6: "#ebe0f8",   # purple (lighter)
    7: "#d8ebf8",   # blue (lighter)
    8: "#ffebd0",   # orange (lighter)
    9: "#e0f5eb",   # mint (lighter)
}
DEFAULT_COLOR = "#e0e0e0"


def _xml_esc(s):
    """Escape special XML characters."""
    return (s.replace("&", "&amp;")
             .replace("<", "&lt;")
             .replace(">", "&gt;")
             .replace('"', "&quot;")
             .replace("'", "&apos;"))


# ---------------------------------------------------------------------------
# Load data
# ---------------------------------------------------------------------------
HERE = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(HERE, "shingetsu_analyzer.json"), encoding="utf-8") as f:
    data = json.load(f)

rows = data["keys"]   # full 5 rows (indices 0-4)

# ---------------------------------------------------------------------------
# Pre-pass: calculate overall width & height
# ---------------------------------------------------------------------------
# Width = max over displayed rows of (stagger + sum of size*U) + PAD on each side
# We only render rows 1, 2, 3 (Q / A / Z rows)
max_row_width = 0
for row_idx in [1, 2, 3]:
    stag = STAGGER[row_idx - 1]
    w = stag
    for key in rows[row_idx]:
        w += key["size"] * U
    if w > max_row_width:
        max_row_width = w

# Legend strip height at bottom
LEGEND_H = 48
# 3 key rows each KH tall, stacked with U spacing vertically (KH + GAP between rows)
ROWS_H = 3 * KH + 2 * GAP   # 3 rows of keys, 2 gaps between them

total_w = int(2 * PAD + max_row_width)
total_h = int(PAD + TTL + ROWS_H + PAD + LEGEND_H + PAD)

# ---------------------------------------------------------------------------
# Build SVG elements
# ---------------------------------------------------------------------------
elems = []   # list of SVG element strings

# --- title ---
title_x = total_w / 2
title_y = PAD + TTL - 6   # baseline
elems.append(
    f'<text x="{title_x}" y="{title_y}" '
    f'text-anchor="middle" font-family="sans-serif" font-size="20" '
    f'font-weight="bold" fill="#333333">新月配列 v1.1.0</text>'
)

# --- key rows ---
for row_seq, row_idx in enumerate([1, 2, 3]):
    stag   = STAGGER[row_seq]                   # horizontal stagger
    base_x = PAD + stag                         # left edge of first key in row
    base_y = PAD + TTL + row_seq * (KH + GAP)   # top edge of key row

    cx = base_x   # running x cursor (left edge of next key)

    for key in rows[row_idx]:
        key_id   = key["id"]
        size     = key["size"]
        legend   = key["legend"]
        finger   = key["finger"]
        kw       = size * U - GAP               # actual pixel width of this key

        if key_id in MODIFIERS:
            # Skip modifier: just advance x
            cx += size * U
            continue

        # Key rectangle
        fill = FINGER_COLORS.get(finger, DEFAULT_COLOR)
        elems.append(
            f'<rect x="{cx}" y="{base_y}" width="{kw}" height="{KH}" '
            f'rx="{R}" ry="{R}" fill="{fill}" stroke="#999999" stroke-width="1"/>'
        )

        # --- legends ---
        # Centre of the key
        center_x = cx + kw / 2
        center_y = base_y + KH / 2

        # legend[0] – base character, large, centred
        base_char = legend[0] if legend else ""
        if base_char:
            nch = len(base_char)
            if nch >= 3:
                fs_base = 9
            elif nch == 2:
                fs_base = 13
            else:
                fs_base = 20
            # vertical centering: dominant-baseline="central" aligns the middle
            elems.append(
                f'<text x="{center_x}" y="{center_y}" '
                f'text-anchor="middle" dominant-baseline="central" '
                f'font-family="sans-serif" font-size="{fs_base}" '
                f'fill="#222222">{_xml_esc(base_char)}</text>'
            )

        # legend[1] – shift layer, larger, top-left
        if len(legend) > 1 and legend[1]:
            shift_char = legend[1]
            sx = cx + 5
            sy = base_y + 13
            elems.append(
                f'<text x="{sx}" y="{sy}" '
                f'text-anchor="start" dominant-baseline="central" '
                f'font-family="sans-serif" font-size="14" '
                f'fill="#0066cc">{_xml_esc(shift_char)}</text>'
            )

        # legend[2] – ☆゛ layer, larger, top-right
        if len(legend) > 2 and legend[2]:
            yaku_char = legend[2]
            yx = cx + kw - 5
            yy = base_y + 13
            elems.append(
                f'<text x="{yx}" y="{yy}" '
                f'text-anchor="end" dominant-baseline="central" '
                f'font-family="sans-serif" font-size="12" '
                f'fill="#6633aa">{_xml_esc(yaku_char)}</text>'
            )

        # Advance cursor
        cx += size * U

# --- bottom legend ---
leg_y = PAD + TTL + ROWS_H + PAD + 10   # top of legend area
leg_items = [
    ("#eeeeee", "単", "そのまま打鍵"),
    ("#cce5ff", "シフト", "★ or ☆ の後に打鍵 (左上の小さな文字)"),
    ("#e2d9f3", "拗音", "☆゛ の後に打鍵 (右上の小さな文字)"),
]

lx = PAD + 8   # running x for legend items (increased left margin)
for (bg, label, desc) in leg_items:
    # Small coloured box
    box_w = 24
    box_h = 18
    elems.append(
        f'<rect x="{lx}" y="{leg_y}" width="{box_w}" height="{box_h}" '
        f'rx="3" ry="3" fill="{bg}" stroke="#aaaaaa" stroke-width="0.75"/>'
    )
    # Label inside box
    elems.append(
        f'<text x="{lx + box_w/2}" y="{leg_y + box_h/2}" '
        f'text-anchor="middle" dominant-baseline="central" '
        f'font-family="sans-serif" font-size="9" font-weight="bold" '
        f'fill="#444444">{_xml_esc(label)}</text>'
    )
    # Description after box
    elems.append(
        f'<text x="{lx + box_w + 5}" y="{leg_y + box_h/2}" '
        f'text-anchor="start" dominant-baseline="central" '
        f'font-family="sans-serif" font-size="10" '
        f'fill="#555555">{_xml_esc(desc)}</text>'
    )
    # Advance: estimate text width (rough: ~6.5 px per char for font-size 10)
    desc_w = len(desc) * 6.5
    lx += box_w + 5 + desc_w + 18   # spacing between legend groups

# ---------------------------------------------------------------------------
# Assemble SVG
# ---------------------------------------------------------------------------
svg_lines = [
    f'<svg xmlns="http://www.w3.org/2000/svg" width="{total_w}" height="{total_h}" '
    f'style="background:#ffffff; font-family:sans-serif;">',
]
for el in elems:
    svg_lines.append("  " + el)
svg_lines.append("</svg>")

svg_content = "\n".join(svg_lines) + "\n"

# ---------------------------------------------------------------------------
# Write output
# ---------------------------------------------------------------------------
out_path = os.path.join(HERE, "shingetsu-layout.svg")
with open(out_path, "w", encoding="utf-8") as f:
    f.write(svg_content)

print(svg_content)
print(f"# Written to: {out_path}", flush=True)
