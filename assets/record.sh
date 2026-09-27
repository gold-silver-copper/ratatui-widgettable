#!/usr/bin/env bash
# Records assets/demo.webp using betamax (https://github.com/joshka/betamax), ffmpeg and img2webp
# (from libwebp). Run from anywhere:
#
#     assets/record.sh
#
# Set BETAMAX to use a betamax binary that is not on the PATH.
set -euo pipefail

cd "$(dirname "$0")/.."
betamax="${BETAMAX:-betamax}"
frames=target/demo-frames

for tool in "$betamax" ffmpeg img2webp; do
    command -v "$tool" >/dev/null || { echo "$tool is required but was not found" >&2; exit 1; }
done

# betamax 0.1.17 fixed the playback timing of video output
version=$("$betamax" --version | cut -d' ' -f2)
if [ "$(printf '%s\n' 0.1.17 "$version" | sort -V | head -1)" != 0.1.17 ]; then
    echo "betamax $version is too old, 0.1.17 or newer is required" >&2
    exit 1
fi

# betamax starts its shell in the home directory, so DEMO_DIR tells the tape where the project is
DEMO_DIR="$PWD" "$betamax" run --quiet assets/demo.tape

# Speed the recording up 2x to keep it short, then convert it into a lossy animated WebP. This is
# much smaller than a GIF or betamax's lossless WebP output, without visibly affecting the text.
rm -rf "$frames"
mkdir -p "$frames"
ffmpeg -v error -i target/demo.mp4 -vf "setpts=PTS/2,fps=10" "$frames/%04d.png"
img2webp -loop 0 -lossy -q 75 -m 4 -d 100 "$frames"/*.png -o assets/demo.webp >/dev/null
echo "wrote assets/demo.webp ($(du -h assets/demo.webp | cut -f1))"
