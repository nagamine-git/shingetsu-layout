#!/usr/bin/env bash
# Build shingetsu-social-preview.png (1280x640, for GitHub's "Social preview") from shingetsu-layout.svg:
# the SVG is drawn at zoom 1.8, its white border is trimmed, and the drawing is centred on a white 1280x640
# canvas (about 65 px left/right and 50 px top/bottom). The title is already inside the SVG.
#
# Needs rsvg-convert (librsvg) and ImageMagick, and a Japanese font from the SVG's font stack
# ('Hiragino Sans', 'BIZ UDPGothic' or 'Noto Sans CJK JP'); the glyph shapes follow the installed font.
# (-strip drops the date chunks so that the same SVG always gives the same bytes.)
# Run it after generate_layout_svg.py:
#   python3 generate_layout_svg.py && ./generate_social_preview.sh
set -euo pipefail
cd "$(dirname "$0")"
rsvg-convert -z 1.8 -b white shingetsu-layout.svg \
  | magick png:- -trim +repage -background white -gravity center -extent 1280x640 -alpha off -strip PNG24:shingetsu-social-preview.png
