#!/usr/bin/env bash
# Render the README screenshots headlessly (needs xvfb-run and a Vulkan driver,
# e.g. mesa-vulkan-drivers / lavapipe on a machine without a GPU).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build
mkdir -p docs
for scene in ${@:-briefing combat eva overload hangar}; do
  HZ_SCENE="$scene" HZ_SHOT="docs/$scene.png" \
    xvfb-run -a -s "-screen 0 1600x900x24" ./target/debug/hangar_zero
done
