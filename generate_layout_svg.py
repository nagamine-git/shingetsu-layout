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
TTL = 60          # title area height (title + breathing room above the keys)
R   = 6           # corner radius of key rect

# Row stagger (ANSI)
STAGGER = [0, 0.25 * U, 0.5 * U]   # for rows index 1, 2, 3 → mapped to 0, 1, 2

# Modifier key IDs to skip (just advance x)
MODIFIERS = {"tab", "caps", "shift", "shiftr", "enter", "backslash", "\\"}

# The layout lives entirely on these 30 keys. 「」 ([ ]) and ' are outside it,
# and / only carries the IME default ・, so they are not drawn / labelled.
THIRTY_KEYS = set("qwertyuiopasdfghjkl;zxcvbnm,./")
IME_DEFAULT_ONLY = {"/"}

FONT = "'Hiragino Sans', 'BIZ UDPGothic', 'Noto Sans CJK JP', sans-serif"

# ゛ drawn as a path: the standalone U+309B glyph sits in a corner of its em box
# and its position differs per font, so text rendering leaves it tiny and
# off-centre. Outline taken from BIZ UDPGothic Bold (SIL OFL 1.1), font units.
DAKUTEN_PATH = ("M215 1282Q150 1445 16 1638L168 1694Q286 1530 368 1343Z"
                "M467 1370Q396 1551 270 1729L415 1778Q530 1638 612 1438Z")
DAKUTEN_BOX = (16, 1282, 612, 1778)   # xmin, ymin, xmax, ymax


def dakuten(cx, cy, height, colour):
    """Return a ゛ centred on (cx, cy) that is `height` px tall."""
    x0, y0, x1, y1 = DAKUTEN_BOX
    k = height / (y1 - y0)
    return (f'<path d="{DAKUTEN_PATH}" fill="{colour}" transform="translate({cx:.1f} {cy:.1f}) '
            f'scale({k:.5f} {-k:.5f}) translate({-(x0 + x1) / 2} {-(y0 + y1) / 2})"/>')


# Text colours: one per layer, reused by the legend so the two always match
INK_BASE  = "#222222"   # layer 1: typed as-is
INK_SHIFT = "#0066cc"   # layer 2: after ☆ / ★
INK_COMBO = "#8a8a8a"   # layer 3: after ☆゛
INK_MUTED = "#999999"   # IME-default punctuation

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
        if key["id"] in THIRTY_KEYS:
            w += key["size"] * U
        elif key["id"] in MODIFIERS and key is rows[row_idx][0]:
            w += key["size"] * U   # leading Tab / Caps / Shift sets the stagger
    w -= GAP
    if w > max_row_width:
        max_row_width = w

# Legend strip height at bottom
LEGEND_H = 64
# 3 key rows each KH tall, stacked with U spacing vertically (KH + GAP between rows)
ROWS_H = 3 * KH + 2 * GAP   # 3 rows of keys, 2 gaps between them

# The skipped Tab / Caps / Shift would leave an empty band on the left; shift
# everything so the left-most drawn key sits on the padding.
LEAD = min(
    STAGGER[i - 1] + (rows[i][0]["size"] * U if rows[i][0]["id"] in MODIFIERS else 0)
    for i in [1, 2, 3]
)
max_row_width -= LEAD

total_w = int(2 * PAD + max_row_width)
total_h = int(PAD + TTL + ROWS_H + PAD + LEGEND_H + PAD)

# ---------------------------------------------------------------------------
# Build SVG elements
# ---------------------------------------------------------------------------
elems = []   # list of SVG element strings

# --- title ---
title_x = PAD
title_y = PAD + 26   # baseline
elems.append(
    f'<text x="{title_x}" y="{title_y}" font-family="{FONT}" font-size="22" '
    f'font-weight="bold" fill="{INK_BASE}">新月配列'
    f'<tspan dx="10" font-size="13" font-weight="normal" fill="{INK_MUTED}">v1.1.0</tspan></text>'
)

# --- key rows ---
for row_seq, row_idx in enumerate([1, 2, 3]):
    stag   = STAGGER[row_seq]                   # horizontal stagger
    base_x = PAD + stag - LEAD                  # left edge of first key in row
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
        if key_id not in THIRTY_KEYS:
            continue
        if key_id in IME_DEFAULT_ONLY:
            legend = [""]

        # Key rectangle (uniform light gray, no finger-based coloring)
        fill = "#f5f5f5"
        # Reduce opacity for keys without shift layers (punctuation, etc.)
        opacity = 'fill-opacity="0.35"' if len(legend) == 1 else ''
        elems.append(
            f'<rect x="{cx}" y="{base_y}" width="{kw}" height="{KH}" '
            f'rx="{R}" ry="{R}" fill="{fill}" {opacity} stroke="#b5b5b5" stroke-width="1"/>'
        )

        # --- legends ---
        # Centre of the key
        center_x = cx + kw / 2
        center_y = base_y + KH / 2

        # legend[0] – base character, large, centred, bold
        base_char = legend[0] if legend else ""
        if base_char.startswith("゛"):
            # 濁点キー (゛ / ゛゛ = ゜ / 小書き): one mark, explained in the legend
            elems.append(dakuten(center_x, center_y + 5, 14, INK_BASE))
        elif base_char:
            nch = len(base_char)
            if nch >= 3:
                fs_base = 13
            elif nch == 2:
                fs_base = 13
            else:
                fs_base = 20
            # Lighter color for keys without shift layers
            base_color = INK_MUTED if len(legend) == 1 else INK_BASE
            # vertical centering: dominant-baseline="central" aligns the middle
            elems.append(
                f'<text x="{center_x}" y="{center_y}" '
                f'text-anchor="middle" dominant-baseline="central" '
                f'font-family="{FONT}" font-size="{fs_base}" font-weight="bold" '
                f'fill="{base_color}">{_xml_esc(base_char)}</text>'
            )

        # legend[1] – shift layer, prominent (larger, bold, darker blue)
        if len(legend) > 1 and legend[1]:
            shift_char = legend[1]
            sx = cx + 5
            sy = base_y + 8  # closer to top to avoid overlap
            elems.append(
                f'<text x="{sx}" y="{sy}" '
                f'text-anchor="start" dominant-baseline="central" '
                f'font-family="{FONT}" font-size="16" font-weight="bold" '
                f'fill="{INK_SHIFT}">{_xml_esc(shift_char)}</text>'
            )

        # legend[2] – ☆゛ layer, subdued (smaller, lighter gray)
        if len(legend) > 2 and legend[2]:
            yaku_char = legend[2]
            yx = cx + kw - 5
            yy = base_y + 7  # smaller, can be closer to top
            elems.append(
                f'<text x="{yx}" y="{yy}" '
                f'text-anchor="end" dominant-baseline="central" '
                f'font-family="{FONT}" font-size="10" '
                f'fill="{INK_COMBO}">{_xml_esc(yaku_char)}</text>'
            )

        # Advance cursor
        cx += size * U

# --- bottom legend ---
# Each sample is a miniature key using the same colour and corner as on the
# keyboard, so the legend reads as "this position/colour means ...".
leg_y = PAD + TTL + ROWS_H + PAD + 6   # top of legend area
leg_items = [
    ("base",  "そ",  INK_BASE,  "そのまま打鍵"),
    ("shift", "ゆ",  INK_SHIFT, "左手側は ☆、右手側は ★ の後に打鍵"),
    ("combo", "ぴょ", INK_COMBO, "☆゛ の後に打鍵"),
]


def _text_w(text, size):
    """Rough width: full-width glyphs = 1em, ASCII = 0.55em."""
    return sum(size if ord(ch) > 0x7F else size * 0.55 for ch in text)


BOX = 24
lx = PAD
for (kind, sample, colour, desc) in leg_items:
    elems.append(
        f'<rect x="{lx}" y="{leg_y}" width="{BOX}" height="{BOX}" '
        f'rx="4" ry="4" fill="#f5f5f5" stroke="#b5b5b5" stroke-width="0.75"/>'
    )
    if kind == "base":
        sx, sy, anchor, fs, weight = lx + BOX / 2, leg_y + BOX / 2, "middle", 14, "bold"
    elif kind == "shift":
        sx, sy, anchor, fs, weight = lx + 3, leg_y + 7, "start", 10, "bold"
    else:
        sx, sy, anchor, fs, weight = lx + BOX - 2, leg_y + 6, "end", 7, "normal"
    elems.append(
        f'<text x="{sx}" y="{sy}" text-anchor="{anchor}" dominant-baseline="central" '
        f'font-family="{FONT}" font-size="{fs}" font-weight="{weight}" '
        f'fill="{colour}">{_xml_esc(sample)}</text>'
    )
    elems.append(
        f'<text x="{lx + BOX + 7}" y="{leg_y + BOX / 2}" '
        f'text-anchor="start" dominant-baseline="central" '
        f'font-family="{FONT}" font-size="11" '
        f'fill="#555555">{_xml_esc(desc)}</text>'
    )
    lx += BOX + 7 + _text_w(desc, 11) + 28

# Second line: how the ゛ key works
ly2 = leg_y + BOX + 10
elems.append(
    f'<rect x="{PAD}" y="{ly2}" width="{BOX}" height="{BOX}" '
    f'rx="4" ry="4" fill="#f5f5f5" stroke="#b5b5b5" stroke-width="0.75"/>'
)
elems.append(dakuten(PAD + BOX / 2, ly2 + BOX / 2, 10, INK_BASE))
elems.append(
    f'<text x="{PAD + BOX + 7}" y="{ly2 + BOX / 2}" '
    f'text-anchor="start" dominant-baseline="central" '
    f'font-family="{FONT}" font-size="11" fill="#555555">'
    f'{_xml_esc("清音の後に 1 回で濁音、2 回で半濁音。親文字の後に 1 回で小書き（ぅ のみ 2 回）")}</text>'
)

# ---------------------------------------------------------------------------
# Assemble SVG
# ---------------------------------------------------------------------------
svg_lines = [
    f'<svg xmlns="http://www.w3.org/2000/svg" width="{total_w}" height="{total_h}" '
    f'style="background:#ffffff;">',
    f'  <rect width="{total_w}" height="{total_h}" fill="#ffffff"/>',
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
