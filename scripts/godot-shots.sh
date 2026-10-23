#!/usr/bin/env bash
# Render the Godot staged scenes to docs/godot/*.png under Xvfb (needs xvfb-run + Mesa).
#   scripts/godot-shots.sh          # all scenes
#   scripts/godot-shots.sh combat   # one scene
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p docs/godot
scenes=${1:-"briefing combat boarded overload hangar"}
for s in $scenes; do
  HZ_SCENE=$s HZ_SHOT="$PWD/docs/godot/$s.png" \
    xvfb-run -a -s "-screen 0 1440x810x24" godot --path godot --rendering-driver opengl3 2>&1 | grep -E "captured|SCRIPT ERROR" || true
done
