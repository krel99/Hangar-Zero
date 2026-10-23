# Notes for agents working on Hangar Zero

Two implementations live here. `godot/` is the **Godot 4.7 port for Android** (the one being
developed); `src/` is the original **Bevy 0.20 + bevy_egui** desktop prototype, kept as the
reference for rules and art. Both paint everything by hand: no image assets, sprites are text
grids, backdrops are generated at startup.

## Godot port (`godot/`)

```sh
godot --headless --path godot --import                       # once after a pull: class_name index
godot --headless --path godot -s tests/test_sim.gd           # rules; 4 seeded autopilot ops must win
godot --headless --path godot -s tests/test_art.gd           # sprite/backdrop bake
xvfb-run -a -s "-screen 0 1440x810x24" godot --path godot --rendering-driver opengl3 -s tests/test_ui.gd
                                                             # synthetic touches/keys through main.tscn
./scripts/godot-shots.sh [scene]                             # staged scenes -> docs/godot/*.png
HZ_SCENE=play HZ_SHOT=out/play.png HZ_FRAMES=8 HZ_EVERY=40 HZ_SEED=7 \
  xvfb-run -a -s "-screen 0 1440x810x24" godot --path godot --rendering-driver opengl3
                                                             # autopilot strip; tile with scripts/strip.py
```

The `godot` binary is the stock Linux editor build (4.7.1); rendering under Xvfb uses Mesa
llvmpipe through the GL Compatibility renderer and takes ~10 s per scene. `play` steps the sim
headlessly between captures, so a whole op renders in under a minute.

Layout (`ui/cockpit.gd`), designed at **480x270** and integer-scaled (`viewport` stretch):
radar 88x88 and the 2x hull diagram with heat/shield/core readouts down the left; the 320x208
camera feed (`ui/feed.gd`) in the middle with the uplink bar and a three-line radio under it;
seven 72x36 stations and PAUSE down the right. `mech.view` is a continuous bearing: drag on
the feed, gyro yaw (`Input.get_gyroscope()` about the gravity axis, untestable here), or
TAB/arrows snapping to camera bearings. `ui/hangar.gd` is the between-ops screen; `ui/main.gd`
owns the campaign, the sim and the screenshot plan; `ui/shots.gd` stages the scenes.

- `sim/` is a line-for-line port of `src/sim.rs` + `campaign.rs` (`rules.gd` holds the enums
  and constants). Keep the two in step when touching balance, or note the divergence.
- `art/sprites.gd` holds the grids, `art/pixel.gd` bakes them to cached textures,
  `art/hull.gd` the section map + detail overlay, `art/city.gd` the panorama and hangar.
- GDScript gotchas met so far: `var x := [a, b][i]` fails to parse (type cannot be inferred;
  annotate), a `-s` script that errors before `quit()` hangs silently (use a load-checker that
  prints `load(path)` results), do not name methods `get`/`_set`, fonts must come from the
  imported `.ttf` (a runtime `FontFile` renders as blobs under llvmpipe), and VT323 is only
  legible at size 12+ on the 480x270 canvas (`Theme2.text` defaults to 12, `pix` labels to 8).
- Keep `docs/godot/*.png` regenerated when the UI changes and **look at them**.

## Desktop prototype (`src/`)

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
- The cockpit is: radar + radio (left), camera feed + station hotbar (centre), hull & core
  gauges (right). No power routing in battle: the energy matrix lives in the hangar
  (`campaign.matrix`, `MATRIX_COST`), and `Mech::new` copies it into `alloc`.
- Balance is checked by `cargo test` (the autopilot must still win most seeded ops in 3–8 min)
  and by looking at a `play` strip; after touching heat, damage or costs run both.
- Text: `pix()` (Silkscreen) for labels and headings, `mono()` (VT323) for numbers and prose.
- Keep the player-facing text minimal; the cockpit should read at a glance.
- Commits go to `claude/lucid-bohr-18n3sd`; the user decides what reaches `main`.
