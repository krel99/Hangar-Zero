# Notes for agents working on Hangar Zero

Bevy 0.20 + bevy_egui 0.43, Rust 2024. The whole game is egui painted by hand; there are no
image assets (sprites are text grids in `src/art.rs`, backdrops are generated at startup).

## Build, test, look

```sh
cargo build                      # ~2.5 min cold, ~15 s warm
cargo test                       # 3 sim tests; the autopilot plays 4 seeded missions headlessly
./scripts/screenshots.sh         # all staged scenes -> docs/*.png (needs xvfb-run + lavapipe)
./scripts/screenshots.sh combat  # one scene
```

Headless rendering uses Xvfb + Mesa lavapipe (`apt install mesa-vulkan-drivers xvfb`). It is
slow (roughly 1 frame/s at 1600x900), and the sim clamps `dt` to 50 ms, so time runs
slower than real time under xvfb. Scenes settle for a couple of seconds before capture.

Always look at the PNG after a UI change (`Read` the file). The layout is designed at
1600x900 and zoomed to fit other window sizes.

## Staged scenes (`src/shots.rs`)

`HZ_SCENE=<name> HZ_SHOT=<path>` stages a moment and captures one frame:

| scene      | what it shows |
|------------|---------------|
| `briefing` | the pre-mission modal |
| `combat`   | mid-fight, railgun on a radar-only target in the blind LEFT quadrant |
| `quad`     | same, with the four-camera wall open |
| `eva`      | pilot out of the chair, fighting boarders in the coolant compartment |
| `overload` | reactor overload countdown, cooked hull |
| `hangar`   | salvage screen after op 2 |

`HZ_SCENE=play` runs the **autopilot** (`src/autopilot.rs`) on a fresh op 1 and saves a frame
every `HZ_EVERY` mission seconds, `HZ_FRAMES` of them, to `<HZ_SHOT>_NN.png`; `HZ_SEED`
picks the RNG seed. Tile them with `python3 -I scripts/strip.py docs/play out.png 4`. Use
it to check that a balance change still plays, not just that it compiles. A full mission is
~5 minutes of sim time; budget ~6 minutes of wall-clock per 4 frames under xvfb.

`scripts/screenshots.sh` is the list of scenes the README images come from; keep them
regenerated when the UI changes.

## Where things live

- `src/sim.rs` — all rules: power bus, heat, cameras and radar estimates (`sensor_fusion`),
  enemies, drones, shells, pilot EVA, outcomes. Tests at the bottom.
- `src/campaign.rs` — persistent state between ops: scars, salvage parts, makers, cameras.
- `src/ui/cockpit.rs` — every cockpit station; `cam_view` is one hull camera.
- `src/ui/hangar.rs`, `src/ui/mod.rs` (palette, fonts, `pbutton`, `panel_frame`).
- `src/art.rs` — sprites, hull map + detail overlay, procedural city and hangar.

## Conventions

- Buttons: `pbutton(ui, rect, "LABEL [KEY]", color, enabled, lit)`; the ` [KEY]` suffix becomes
  a small key tag.
- Text: `pix()` (Silkscreen) for labels and headings, `mono()` (VT323) for numbers and prose.
- Keep the player-facing text minimal; the cockpit should read at a glance.
- Commits go to `claude/lucid-bohr-18n3sd`; the user decides what reaches `main`.
