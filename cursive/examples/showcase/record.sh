#!/bin/sh
# Records the showcase example in demo mode, and renders it as an animated
# (lossless) WebP: about 40% of the size of the same GIF.
#
# Requires `asciinema` (3.0+), `agg` and `ffmpeg`.
#
# Usage: ./record.sh [output.webp]
set -eu

cd "$(dirname "$0")/../../.."
output="${1:-doc/showcase.webp}"
cast="$(mktemp --suffix .cast)"
gif="$(mktemp --suffix .gif)"
trap 'rm -f "$cast" "$gif"' EXIT

cargo build --release --example showcase

COLORTERM=truecolor TERM=xterm-256color asciinema rec \
    --quiet --overwrite --headless --window-size 120x36 \
    --command "target/release/examples/showcase --demo" \
    "$cast"

# A font size giving whole-pixel cells (20px: 12x28) keeps the rounded corners
# aligned with the lines.
agg --font-size 20 --fps-cap 15 "$cast" "$gif"
# Without a frame after it, the last frame would lose its pause before the
# loop: repeat it briefly.
ffmpeg -y -loglevel error -i "$gif" \
    -vf tpad=stop_mode=clone:stop_duration=0.1 \
    -c:v libwebp_anim -lossless 1 -compression_level 6 -loop 0 "$output"
echo "Wrote $output"
