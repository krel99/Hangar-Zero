//! Scripted screenshot scenes, used to produce the README images headlessly:
//!
//! `HZ_SCENE=combat HZ_SHOT=docs/combat.png cargo run`
//!
//! Each scene stages a moment of an encounter (enemy positions, damage, radio
//! history), lets the real simulation run for a few seconds, then captures the
//! primary window and exits.
//!
//! `HZ_SCENE=play` instead hands the mech to the autopilot at 4x speed and saves a
//! frame every `HZ_EVERY` mission seconds (`HZ_FRAMES` of them) to `<HZ_SHOT>_NN.png`,
//! so a whole mission can be reviewed as a strip (`scripts/strip.py` tiles them).

use std::f32::consts::TAU;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use crate::campaign::{Campaign, MissionReport, PartId};
use crate::rng::Rng;
use crate::sim::*;
use crate::ui::hangar::HangarState;
use crate::AppState;

#[derive(Resource)]
pub struct ShotPlan {
    pub scene: String,
    pub path: String,
    pub seed: u64,
    pub frames: u32,
    pub every: f32,
    taken: u32,
    staged_at: Option<f32>,
    shot_at: Option<f32>,
}

impl ShotPlan {
    pub fn from_env() -> Option<Self> {
        let scene = std::env::var("HZ_SCENE").ok()?;
        let path = std::env::var("HZ_SHOT").unwrap_or_else(|_| format!("{scene}.png"));
        let frames = std::env::var("HZ_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(8);
        let every = std::env::var("HZ_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(25.0);
        let seed = std::env::var("HZ_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(1987);
        Some(Self { scene, path, seed, frames, every, taken: 0, staged_at: None, shot_at: None })
    }

    pub fn is_play(&self) -> bool {
        self.scene == "play"
    }

    fn settle(&self) -> f32 {
        match self.scene.as_str() {
            "briefing" | "hangar" => 1.5,
            "eva" => 3.0,
            "quad" => 2.4,
            _ => 2.4,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_shot_plan(
    time: Res<Time<Real>>,
    mut plan: ResMut<ShotPlan>,
    state: Res<State<AppState>>,
    mut commands: Commands,
    mut rng: ResMut<Rng>,
    mut campaign: ResMut<Campaign>,
    mut hangar: ResMut<HangarState>,
    mech: Option<ResMut<Mech>>,
    mission: Option<ResMut<Mission>>,
    mut radio: ResMut<Radio>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if now < 1.0 {
        return;
    }
    if plan.is_play() {
        let (Some(mut mission), Some(_)) = (mission, mech) else { return };
        mission.briefing = false;
        let done = mission.outcome.is_some();
        if (mission.time >= plan.taken as f32 * plan.every || done) && plan.taken < plan.frames && plan.shot_at.is_none_or(|t| now - t > 0.5) {
            let path = format!("{}_{:02}.png", plan.path.trim_end_matches(".png"), plan.taken);
            commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
            info!("capturing {path} at T+{:.0}", mission.time);
            plan.taken += 1;
            plan.shot_at = Some(now);
            if done {
                plan.taken = plan.frames;
            }
        }
        if plan.taken >= plan.frames && plan.shot_at.is_some_and(|t| now - t > 1.5) {
            exit.write(AppExit::Success);
        }
        return;
    }
    if plan.staged_at.is_none() {
        let ready = match plan.scene.as_str() {
            "hangar" => *state.get() == AppState::Hangar,
            _ => *state.get() == AppState::Cockpit && mech.is_some(),
        };
        if !ready {
            return;
        }
        match plan.scene.as_str() {
            "hangar" => stage_hangar(&mut campaign, &mut rng, &mut hangar),
            "briefing" => {}
            scene => {
                let (Some(mut mech), Some(mut mission)) = (mech, mission) else { return };
                stage_combat(scene, &mut commands, &mut rng, &mut mech, &mut mission, &mut radio);
            }
        }
        plan.staged_at = Some(now);
        return;
    }
    let staged = plan.staged_at.unwrap();
    if plan.shot_at.is_none() && now - staged > plan.settle() {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(plan.path.clone()));
        plan.shot_at = Some(now);
        info!("capturing {}", plan.path);
    }
    if plan.shot_at.is_some_and(|t| now - t > 1.5) {
        exit.write(AppExit::Success);
    }
}

fn deg(d: f32) -> f32 {
    d.to_radians().rem_euclid(TAU)
}

fn stage_combat(scene: &str, commands: &mut Commands, _rng: &mut Rng, mech: &mut Mech, mission: &mut Mission, radio: &mut Radio) {
    mission.briefing = false;
    mission.time = 131.0;
    mission.next_wave = 5;
    radio.time = 0.0;
    radio.log.clear();
    let script: [(f32, Voice, &str); 12] = [
        (0.0, Voice::System, "BASTION OS v7.31 — COLD START COMPLETE. 3 OF 9 SUBSYSTEMS REPORT FAULTS."),
        (2.0, Voice::Control, "Bastion, City Control. Uplink mast is dark. Get it back up and hold the line."),
        (31.0, Voice::Control, "Bastion, seismic thump bearing 302. Something heavy is setting up out there."),
        (47.0, Voice::Pilot, "Scout down."),
        (56.0, Voice::Control, "Hive signature bearing 118. Carrier class. Don't let the ticks latch on."),
        (66.0, Voice::Enemy, "[INTERCEPT] CRB-03: braced. Ranging on the walker."),
        (74.0, Voice::Chief, "Boarding drones! Point defence on, or they'll chew through the hull."),
        (88.0, Voice::System, "HULL CONTACT: boarding drone latched onto COOLANT LINES."),
        (88.5, Voice::System, "PARASITIC DRAW DETECTED: -6 UNITS"),
        (102.0, Voice::Control, "Uplink took a hit! Restore progress lost."),
        (119.0, Voice::Chief, "Core's running hot. Ease off the rail or give me more cooling."),
        (128.0, Voice::Pilot, "Rail's charged. Crab first, then the jackals."),
    ];
    for (t, v, s) in script {
        radio.log.push(RadioMsg { time: t, voice: v, text: s.into() });
    }
    radio.time = mission.time;
    mission.serial = 7;

    let mk = |kind: EnemyKind, serial: u32, a: f32, d: f32, state: EnemyState, hp: f32, max: f32, orbit: f32| Enemy {
        kind,
        serial,
        hp,
        max_hp: max,
        angle: a,
        dist: d,
        hold_dist: d,
        state,
        timer: 1.5,
        orbit,
        stun: 0.0,
        drones: 2,
        hit_flash: 0.0,
        reveal: 0.0,
        est_angle: a,
        est_dist: d,
        err: 0.0,
        covered: false,
        last_sweep: -10.0,
    };

    match scene {
        "overload" => {
            commands.spawn(mk(EnemyKind::Scout, 5, deg(28.0), 180.0, EnemyState::Engaged, 41.0, 60.0, 1.0));
            commands.spawn(mk(EnemyKind::Scout, 6, deg(64.0), 205.0, EnemyState::Engaged, 60.0, 60.0, -1.0));
            commands.spawn(mk(EnemyKind::Artillery, 3, deg(36.0), 560.0, EnemyState::Engaged, 61.0, 200.0, 1.0));
            commands.spawn(mk(EnemyKind::Carrier, 4, deg(118.0), 340.0, EnemyState::Engaged, 105.0, 150.0, 1.0));
            commands.spawn(Boarder { section: Section::Coolant, age: 10.0 });
            commands.spawn(Boarder { section: Section::Reactor, age: 4.0 });
            commands.spawn(Shell { target: Section::Reactor, t: 1.0, flight: 6.5, angle: deg(36.0) });
            mech.heat = 112.0;
            mech.overload = Some(OVERLOAD_TIME - 3.5);
            mech.alloc = [5.0, 10.0, 40.0, 25.0, 15.0, 15.0, 10.0];
            mech.hp[Section::Coolant.idx()] = 22.0;
            mech.hp[Section::Reactor.idx()] = 48.0;
            mech.hp[Section::Cockpit.idx()] = 61.0;
            mech.hp[Section::LeftLeg.idx()] = 0.0;
            mech.hp[Section::Sensors.idx()] = 52.0;
            mech.hp[Section::Uplink.idx()] = 71.0;
            mech.repair_target = Section::Coolant;
            mech.rail_charge = 0.55;
            mech.heat_peak = 112.0;
            radio.log.push(RadioMsg { time: 129.0, voice: Voice::System, text: "!! REACTOR OVERLOAD !! CONTAINMENT FAILURE IMMINENT.".into() });
            radio.log.push(RadioMsg { time: 129.2, voice: Voice::Chief, text: "Vent it, SCRAM it, or get out of there!".into() });
        }
        "eva" => {
            commands.spawn(mk(EnemyKind::Carrier, 4, deg(300.0), 345.0, EnemyState::Engaged, 96.0, 150.0, 1.0));
            commands.spawn(mk(EnemyKind::Scout, 6, deg(340.0), 470.0, EnemyState::Approach, 60.0, 60.0, -1.0));
            commands.spawn(Boarder { section: Section::Coolant, age: 10.0 });
            commands.spawn(Boarder { section: Section::Coolant, age: 6.0 });
            commands.spawn(Boarder { section: Section::Railgun, age: 3.0 });
            commands.spawn(Drone { angle: deg(300.0), dist: 220.0, hp: 10.0 });
            mech.pilot = Pilot::Working { at: Section::Coolant, t: 0.4 };
            mech.heat = 58.0;
            mech.hp[Section::Coolant.idx()] = 41.0;
            mech.hp[Section::Railgun.idx()] = 64.0;
            mech.hp[Section::Sensors.idx()] = 80.0;
            mech.hp[Section::Uplink.idx()] = 44.0;
            mech.repair_target = Section::Uplink;
            radio.log.push(RadioMsg { time: 129.0, voice: Voice::Pilot, text: "Leaving the chair. Crawling to the coolant lines.".into() });
        }
        _ => {
            // "combat" / "quad": the general cockpit view mid-fight. The artillery sits in the
            // dead LEFT quadrant, so it is a radar-only blip; the scouts are on camera.
            if scene == "quad" {
                mech.quad_view = true;
            }
            commands.spawn(mk(EnemyKind::Scout, 5, deg(22.0), 230.0, EnemyState::Engaged, 41.0, 60.0, 1.0));
            commands.spawn(mk(EnemyKind::Scout, 6, deg(62.0), 330.0, EnemyState::Approach, 60.0, 60.0, -1.0));
            commands.spawn(mk(EnemyKind::Artillery, 3, deg(292.0), 440.0, EnemyState::Engaged, 141.0, 200.0, 1.0));
            commands.spawn(mk(EnemyKind::Carrier, 4, deg(118.0), 345.0, EnemyState::Engaged, 96.0, 150.0, 1.0));
            commands.spawn(Drone { angle: deg(118.0), dist: 150.0, hp: 10.0 });
            commands.spawn(Boarder { section: Section::Coolant, age: 10.0 });
            commands.spawn(Shell { target: Section::LeftLeg, t: 0.6, flight: 6.0, angle: deg(292.0) });
            mech.heat = 74.0;
            mech.heat_peak = 88.0;
            mech.hp[Section::LeftLeg.idx()] = 31.0;
            mech.hp[Section::Sensors.idx()] = 58.0;
            mech.hp[Section::Coolant.idx()] = 52.0;
            mech.hp[Section::Cockpit.idx()] = 77.0;
            mech.hp[Section::ShieldGen.idx()] = 84.0;
            mech.hp[Section::Railgun.idx()] = 92.0;
            mech.hp[Section::Uplink.idx()] = 47.0;
            mech.alloc = [10.0, 15.0, 25.0, 15.0, 15.0, 15.0, 20.0];
            mech.rail_charge = 0.82;
            mech.shield = 40.0;
            mech.repair_target = Section::Uplink;
            mech.kills = 2;
        }
    }
    // Target the first scout so camera + radar show a lock.
    mech.target = None;
    let want_radar_only = matches!(scene, "combat" | "quad");
    commands.queue(move |world: &mut World| {
        let mut q = world.query::<(Entity, &Enemy)>();
        // Combat/quad lock the artillery hiding in the blind quadrant (radar-only aim);
        // the other scenes lock whatever is closest.
        let pick = q
            .iter(world)
            .find(|(_, e)| if want_radar_only { e.serial == 3 } else { e.serial == 5 || (e.serial == 4 && e.kind == EnemyKind::Carrier) })
            .map(|(e, _)| e);
        if let Some(e) = pick {
            world.resource_mut::<Mech>().target = Some(e);
        }
    });
}

fn stage_hangar(campaign: &mut Campaign, rng: &mut Rng, hangar: &mut HangarState) {
    campaign.mission = 3;
    campaign.install(PartId::CapacitorBank);
    campaign.install(PartId::NavyRadiator);
    for (s, v) in [(Section::LeftLeg, 72.0), (Section::Sensors, 81.0), (Section::Cockpit, 90.0), (Section::Reactor, 88.0)] {
        campaign.max_hp[s.idx()] = v;
    }
    campaign.last_report = Some(MissionReport {
        number: 2,
        time: 284.0,
        kills: 9,
        heat_peak: 104.0,
        scars: vec![(Section::LeftLeg, 14.0), (Section::Sensors, 9.0), (Section::Reactor, 6.0)],
    });
    campaign.roll_salvage(rng);
    hangar.chosen = None;
}
