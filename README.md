# HANGAR ZERO

*You are the last pilot of a decommissioned walking fortress defending a dying city.*

A cockpit-only mech prototype built with **Bevy 0.20** and **egui** (via `bevy_egui`). You never
see the mech from outside during combat. You get a radar, an external camera, a reactor bus with
more demand than supply, a heat gauge that matters, a damage diagram, and a radio full of people
who need you to hold on a little longer.

The question isn't "can I aim?" It's whether you keep firing the railgun and risk a meltdown,
or cut power and sit still while the scouts close in.

![Cockpit mid-fight](docs/combat.png)

## What's in the prototype

| Spec item | Where it is |
|---|---|
| One cockpit screen with radar, power allocation, heat, hull diagram, two weapons | `src/ui/cockpit.rs` |
| Three enemy types: fast scout, artillery platform, boarding drone carrier | `src/sim.rs` (`EnemyKind`) |
| One encounter: defend the mech while restoring a downed subsystem (the city uplink) | `src/sim.rs` (`spawn_waves`, repair drones) |
| One failure state: reactor overload | `src/sim.rs` (`power_and_heat`, `check_outcome`) |
| One hangar screen with five salvage choices | `src/ui/hangar.rs`, `src/campaign.rs` |

All the pixel art is hand-authored as text grids (`src/art.rs`). The city panorama and the
hangar are painted procedurally at startup, so the game ships no image assets. Fonts are
[VT323](https://fonts.google.com/specimen/VT323) and [Silkscreen](https://fonts.google.com/specimen/Silkscreen)
(SIL OFL, in `assets/fonts`).

### Power routing is the core verb

The reactor makes **100 units**. Seven buses want **205**:

| Bus | Cap | Effect |
|---|---|---|
| LOCO | 30 | Evasive stepping. Dodges artillery and scout fire. A destroyed leg cuts it hard. |
| SENS | 20 | Radar detection and ID range. Artillery parks *past* your default radar. |
| RAIL | 40 | Capacitor charge rate. Each shot is +22 heat. |
| A/C | 25 | Autocannon damage, point defence against boarding drones. Heats while firing. |
| SHLD | 35 | Shield capacity and regen. Absorbed hits turn into heat. |
| REPR | 25 | Repair drones work on **one** selected subsystem, or restore the uplink. |
| COOL | 30 | Pulls heat out of the core. Scales with coolant-line health. |

You can **overdrive** the bus up to 130% of available output. Past the white line every extra
unit is heat. Past the red line you're in brownout and every system gets a proportional share.
Boarding drones bolted to the hull feed on the bus (parasitic draw). A damaged reactor makes less.

### Heat and the failure state

Heat comes from the core load, overdrive, railgun shots, autocannon fire, shield absorption,
a damaged reactor, leaking coolant, and failed interlocks after a cockpit breach. Above
**100** the containment countdown starts (12 s). Get below 92 or lose the machine. Your outs:

* **VENT** (`V`) dumps heat fast, but your shields drop for 6 s and the coolant lines take damage.
* **SCRAM** (`X`) kills every bus for 6 s. You're cold, blind and defenceless.
* **EJECT** unlocks during the countdown. You live, Bastion doesn't.

![Reactor overload](docs/overload.png)

### Localized damage

Ten sections, each with its own integrity, and each one changes how you play:

* **Legs**: evasion. Losing one leg cuts it by 60%.
* **Sensor array**: radar range, contact ID, camera static, ghost contacts.
* **Coolant lines**: cooling efficiency. Below 35% they leak heat.
* **Cockpit**: below 25% the canopy is breached and a 45 s countdown starts. When it runs
  out the reactor interlocks fail and the core heats permanently.
* **Weapon mounts**: charge rate and damage.
* **Reactor**: output.
* **Shield emitters**: capacity.
* **City uplink**: the objective. Artillery can knock restore progress back down.

### Enemies

* **Jackal scout**: fast, circles at 150–210 m and chips random subsystems. The autocannon's job.
* **Mortar Crab**: crawls to ~680 m, braces, and lobs shells with a 4.5 s flight time. The target
  section blinks on the hull diagram. Moving (LOCO) can dodge them. You need SENS power to even
  get a railgun lock at that range. Its muzzle flash gives away a bearing on the radar.
* **Tick Carrier**: hangs at ~340 m and launches boarding drones. Point defence shoots them down
  in flight. The ones that latch on chew their subsystem and steal reactor power.

### Leaving the chair

Click **LEAVE CHAIR**, a hull row's **GO**, or a compartment in the **interior cutaway** and the
pilot crawls through the mech. Once there you can pry boarders off by hand (2.2 s each) or do
hands-on repair (+5/s, no reactor power needed). While you're out of the seat the weapons hold
fire and the power controls are locked.

![Pilot out of the chair fighting boarders](docs/eva.png)

### The hangar

Each operation leaves **permanent scars**: max integrity lost in proportion to the damage taken.
Then you pick **one of five salvage parts**. Most come from other manufacturers or from the
enemy (Kessler-Volt capacitors, a navy radiator, a Jackal sensor dish, a Hive-grafted reactor
sleeve...). Each has a real upside and a real cost. Installing a part:

* re-stamps that cockpit station's faceplate with the new maker's name and colour,
* can change the readout itself (the Jackal dish turns the round radar into a square-grid scope
  with enemy firmware ghosts),
* lowers the **ORIGINAL FRAME** percentage. Her description drifts from *"WF-7 BASTION, as she left
  the factory"* toward *"the Chimera"*.

Or you take the **Bastion Original Spares** and try to keep her the machine she was.

![The hangar](docs/hangar.png)

![Briefing](docs/briefing.png)

## Controls

| Input | Action |
|---|---|
| Click radar blip / contact row, `Tab` | Select target |
| `Space` / FIRE | Fire railgun (needs full charge and a target inside radar range) |
| Drag faders, `+`/`-`, `1`–`7` + `↑`/`↓` | Route power (`Shift` for ±1) |
| ASSAULT / RESTORE | Routing presets |
| `V` / `X` / `E` | Vent / SCRAM / ECM burst (fries nearby drones and boarders, jams scouts) |
| Click hull diagram or row | Send repair drones there |
| GO / LEAVE CHAIR / interior click | Crawl to a compartment |
| `P` / `Esc` | Pause |

## Building

```sh
cargo run
```

On Linux you'll need the usual Bevy dependencies (`libudev-dev`, `libasound2-dev`,
`libxkbcommon-dev`, a Vulkan driver). The window opens at 1600×900 and the UI zooms to fit
any size.

```sh
cargo test           # simulation tests, including an autopilot that plays whole missions
```

### Screenshots

The images in `docs/` are real captures from the game, taken headlessly. Each `HZ_SCENE`
stages a moment of an encounter (enemy positions, damage, radio history), lets the real
simulation run for a couple of seconds, then saves the window:

```sh
./scripts/screenshots.sh                  # all scenes, uses xvfb-run
HZ_SCENE=combat HZ_SHOT=out.png cargo run # one scene: briefing | combat | eva | overload | hangar
```

## Code map

```
src/main.rs         app, states (Boot → Cockpit ⇄ Hangar), mission start / flow
src/sim.rs          subsystems, power bus, heat, enemies, drones, shells, pilot EVA
src/campaign.rs     scars, salvage catalogue, makers, originality
src/art.rs          pixel sprites (text grids), hull map, procedural city + hangar
src/ui/mod.rs       fonts, palette, riveted panels, pixel buttons and LED bars
src/ui/cockpit.rs   every cockpit station
src/ui/hangar.rs    hangar screen
src/shots.rs        scripted screenshot scenes
```
