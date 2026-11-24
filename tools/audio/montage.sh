#!/usr/bin/env bash
# Render the review montages (docs/audio/montages/*.wav): six scripted
# situations played through the game's own mixer with no sound device, and
# print the measured level of each and of each bus.
#
# usage: tools/audio/montage.sh [out-dir]
set -euo pipefail
cd "$(dirname "$0")/../.."
exec cargo run -q -p dyno-client --example audio_montage -- "$@"
