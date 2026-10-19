//! Combat simulation: the mech's subsystems, reactor power routing, heat, enemies,
//! boarding drones, artillery shells and the pilot leaving the chair.

use std::f32::consts::{PI, TAU};

use bevy::prelude::*;

use crate::campaign::{Campaign, Mods};
use crate::rng::Rng;

// ---------------------------------------------------------------------------
// Subsystems
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Section {
    Reactor,
    Cockpit,
    Sensors,
    Coolant,
    LeftLeg,
    RightLeg,
    Railgun,
    Autocannon,
    ShieldGen,
    Uplink,
}

impl Section {
    pub const ALL: [Section; 10] = [
        Section::Reactor,
        Section::Cockpit,
        Section::Sensors,
        Section::Coolant,
        Section::LeftLeg,
        Section::RightLeg,
        Section::Railgun,
        Section::Autocannon,
        Section::ShieldGen,
        Section::Uplink,
    ];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Section::Reactor => "REACTOR",
            Section::Cockpit => "COCKPIT",
            Section::Sensors => "SENSOR ARRAY",
            Section::Coolant => "COOLANT LINES",
            Section::LeftLeg => "LEFT LEG",
            Section::RightLeg => "RIGHT LEG",
            Section::Railgun => "RAILGUN MOUNT",
            Section::Autocannon => "AUTOCANNON MOUNT",
            Section::ShieldGen => "SHIELD EMITTERS",
            Section::Uplink => "CITY UPLINK",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Section::Reactor => "RCT",
            Section::Cockpit => "CPT",
            Section::Sensors => "SNS",
            Section::Coolant => "CLT",
            Section::LeftLeg => "L-LEG",
            Section::RightLeg => "R-LEG",
            Section::Railgun => "RAIL",
            Section::Autocannon => "A/C",
            Section::ShieldGen => "SHLD",
            Section::Uplink => "UPLNK",
        }
    }

    /// Interior deck (0 = top of the mast, 4 = hips). Used for crawl times.
    pub fn deck(self) -> u32 {
        match self {
            Section::Uplink | Section::Sensors => 0,
            Section::Cockpit => 1,
            Section::Railgun | Section::Autocannon | Section::ShieldGen => 2,
            Section::Reactor | Section::Coolant => 3,
            Section::LeftLeg | Section::RightLeg => 4,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sys {
    Locomotion,
    Sensors,
    Railgun,
    Autocannon,
    Shields,
    Repair,
    Cooling,
}

impl Sys {
    pub const ALL: [Sys; 7] = [
        Sys::Locomotion,
        Sys::Sensors,
        Sys::Railgun,
        Sys::Autocannon,
        Sys::Shields,
        Sys::Repair,
        Sys::Cooling,
    ];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Sys::Locomotion => "LOCO",
            Sys::Sensors => "SENS",
            Sys::Railgun => "RAIL",
            Sys::Autocannon => "A/C",
            Sys::Shields => "SHLD",
            Sys::Repair => "REPR",
            Sys::Cooling => "COOL",
        }
    }

    pub fn long_name(self) -> &'static str {
        match self {
            Sys::Locomotion => "Locomotion: evasive stepping, dodges shells",
            Sys::Sensors => "Sensors: radar range and contact ID",
            Sys::Railgun => "Railgun: capacitor charge rate",
            Sys::Autocannon => "Autocannon: rate of fire / point defence",
            Sys::Shields => "Shield array: capacity and regen",
            Sys::Repair => "Repair drones: fix the selected subsystem",
            Sys::Cooling => "Cooling: pulls heat out of the core",
        }
    }

    /// Maximum units the bus to this system can carry.
    pub fn cap(self) -> f32 {
        match self {
            Sys::Locomotion => 30.0,
            Sys::Sensors => 20.0,
            Sys::Railgun => 40.0,
            Sys::Autocannon => 25.0,
            Sys::Shields => 35.0,
            Sys::Repair => 25.0,
            Sys::Cooling => 30.0,
        }
    }

    pub fn default_alloc(self) -> f32 {
        match self {
            Sys::Locomotion => 10.0,
            Sys::Sensors => 10.0,
            Sys::Railgun => 25.0,
            Sys::Autocannon => 10.0,
            Sys::Shields => 15.0,
            Sys::Repair => 15.0,
            Sys::Cooling => 15.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pilot {
    Seat,
    Crawling { to: Section, t: f32, dur: f32 },
    Working { at: Section, t: f32 },
    Returning { from: Section, t: f32, dur: f32 },
}

impl Pilot {
    pub fn in_seat(&self) -> bool {
        matches!(self, Pilot::Seat)
    }
}

pub fn crawl_time(s: Section) -> f32 {
    2.0 + 0.9 * (s.deck() as f32 - Section::Cockpit.deck() as f32).abs()
}

pub const OVERDRIVE: f32 = 1.3;
/// Heat per second for every unit drawn past the reactor's rated output.
pub const OVERDRIVE_HEAT: f32 = 0.25;
pub const HEAT_CRIT: f32 = 100.0;
pub const HEAT_MAX: f32 = 130.0;
pub const OVERLOAD_TIME: f32 = 12.0;
pub const BREACH_TIME: f32 = 45.0;
pub const AC_RANGE: f32 = 420.0;
/// Uplink restore percent per second per unit of repair power.
pub const UPLINK_RATE: f32 = 0.018;

#[derive(Resource)]
pub struct Mech {
    pub hp: [f32; 10],
    pub max_hp: [f32; 10],
    pub alloc: [f32; 7],
    pub heat: f32,
    pub heat_peak: f32,
    pub shield: f32,
    pub shield_hit_t: f32,
    pub rail_charge: f32,
    pub rail_auto: bool,
    pub weapons_free: bool,
    pub point_defence: bool,
    pub target: Option<Entity>,
    pub repair_target: Section,
    pub vent_t: f32,
    pub vent_cd: f32,
    pub shields_down_t: f32,
    pub scram_t: f32,
    pub scram_cd: f32,
    pub ecm_cd: f32,
    pub overload: Option<f32>,
    pub breach: Option<f32>,
    pub interlocks_failed: bool,
    pub pilot: Pilot,
    pub sweep: f32,
    pub ac_firing: bool,
    pub ac_burst_t: f32,
    pub hit_flash: f32,
    pub shake: f32,
    pub kills: u32,
    /// Snapshot of the derived numbers for the UI.
    pub stats: Derived,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Derived {
    pub available: f32,
    pub parasitic: f32,
    pub draw: f32,
    pub supply: f32,
    pub eff: [f32; 7],
    pub radar_range: f32,
    pub id_range: f32,
    pub evasion: f32,
    pub cooling: f32,
    pub heat_in: f32,
    pub heat_out: f32,
    pub shield_cap: f32,
    pub rail_rate: f32,
    pub ac_dps: f32,
    pub repair_rate: f32,
}

impl Mech {
    pub fn new(campaign: &Campaign) -> Self {
        let mut hp = campaign.max_hp;
        hp[Section::Uplink.idx()] = 0.0;
        let mut alloc = [0.0; 7];
        for s in Sys::ALL {
            alloc[s.idx()] = s.default_alloc();
        }
        Self {
            hp,
            max_hp: campaign.max_hp,
            alloc,
            heat: 22.0,
            heat_peak: 22.0,
            shield: 30.0,
            shield_hit_t: 0.0,
            rail_charge: 0.4,
            rail_auto: false,
            weapons_free: true,
            point_defence: true,
            target: None,
            repair_target: Section::Uplink,
            vent_t: 0.0,
            vent_cd: 0.0,
            shields_down_t: 0.0,
            scram_t: 0.0,
            scram_cd: 0.0,
            ecm_cd: 0.0,
            overload: None,
            breach: None,
            interlocks_failed: false,
            pilot: Pilot::Seat,
            sweep: 0.0,
            ac_firing: false,
            ac_burst_t: 0.0,
            hit_flash: 0.0,
            shake: 0.0,
            kills: 0,
            stats: Derived::default(),
        }
        .with_stats(&campaign.mods)
    }

    fn with_stats(mut self, m: &Mods) -> Self {
        self.stats = derive(&self, m, 0.0);
        self
    }

    pub fn frac(&self, s: Section) -> f32 {
        (self.hp[s.idx()] / self.max_hp[s.idx()].max(1.0)).clamp(0.0, 1.0)
    }

    /// How well a section works: destroyed sections stop, damaged ones limp along.
    pub fn eff(&self, s: Section) -> f32 {
        if self.hp[s.idx()] <= 0.0 {
            0.0
        } else {
            0.3 + 0.7 * self.frac(s)
        }
    }

    pub fn uplink(&self) -> f32 {
        self.hp[Section::Uplink.idx()]
    }

    pub fn damage(&mut self, s: Section, amount: f32) {
        let i = s.idx();
        self.hp[i] = (self.hp[i] - amount).max(0.0);
    }
}

// ---------------------------------------------------------------------------
// Enemies & projectiles
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyKind {
    Scout,
    Artillery,
    Carrier,
}

impl EnemyKind {
    pub fn name(self) -> &'static str {
        match self {
            EnemyKind::Scout => "JACKAL SCOUT",
            EnemyKind::Artillery => "MORTAR CRAB",
            EnemyKind::Carrier => "TICK CARRIER",
        }
    }

    pub fn callsign(self) -> &'static str {
        match self {
            EnemyKind::Scout => "JKL",
            EnemyKind::Artillery => "CRB",
            EnemyKind::Carrier => "TCK",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnemyState {
    Approach,
    Deploying,
    Engaged,
}

#[derive(Component, Clone, Debug)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub serial: u32,
    pub hp: f32,
    pub max_hp: f32,
    pub angle: f32,
    pub dist: f32,
    pub hold_dist: f32,
    pub state: EnemyState,
    pub timer: f32,
    pub orbit: f32,
    pub stun: f32,
    pub drones: u32,
    pub hit_flash: f32,
    pub reveal: f32,
}

/// Boarding drone in flight.
#[derive(Component, Clone, Debug)]
pub struct Drone {
    pub angle: f32,
    pub dist: f32,
    pub hp: f32,
}

/// Boarding drone clamped onto a subsystem.
#[derive(Component, Clone, Debug)]
pub struct Boarder {
    pub section: Section,
    pub age: f32,
}

/// Artillery shell in the air.
#[derive(Component, Clone, Debug)]
pub struct Shell {
    pub target: Section,
    pub t: f32,
    pub flight: f32,
    pub angle: f32,
}

// ---------------------------------------------------------------------------
// Mission, radio and effects
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Victory,
    Meltdown,
    Ejected,
}

#[derive(Resource)]
pub struct Mission {
    pub number: u32,
    pub time: f32,
    pub paused: bool,
    pub briefing: bool,
    pub outcome: Option<Outcome>,
    pub next_wave: usize,
    pub serial: u32,
    pub damage_taken: [f32; 10],
    pub warned: Vec<&'static str>,
}

impl Mission {
    pub fn new(number: u32) -> Self {
        Self {
            number,
            time: 0.0,
            paused: false,
            briefing: true,
            outcome: None,
            next_wave: 0,
            serial: 1,
            damage_taken: [0.0; 10],
            warned: Vec::new(),
        }
    }

    pub fn running(&self) -> bool {
        !self.paused && !self.briefing && self.outcome.is_none()
    }

    fn warn_once(&mut self, key: &'static str) -> bool {
        if self.warned.contains(&key) {
            false
        } else {
            self.warned.push(key);
            true
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Voice {
    Control,
    Chief,
    Pilot,
    Enemy,
    System,
}

#[derive(Clone, Debug)]
pub struct RadioMsg {
    pub time: f32,
    pub voice: Voice,
    pub text: String,
}

#[derive(Resource, Default)]
pub struct Radio {
    pub log: Vec<RadioMsg>,
    pub time: f32,
}

impl Radio {
    pub fn say(&mut self, voice: Voice, text: impl Into<String>) {
        self.log.push(RadioMsg { time: self.time, voice, text: text.into() });
        if self.log.len() > 60 {
            self.log.remove(0);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FxKind {
    Rail,
    Tracer,
    Explosion,
    Muzzle,
    ShellImpact,
    Ecm,
}

#[derive(Clone, Copy, Debug)]
pub struct Fx {
    pub kind: FxKind,
    pub angle: f32,
    pub dist: f32,
    pub t: f32,
    pub ttl: f32,
}

#[derive(Resource, Default)]
pub struct Effects {
    pub list: Vec<Fx>,
}

impl Effects {
    pub fn add(&mut self, kind: FxKind, angle: f32, dist: f32, ttl: f32) {
        self.list.push(Fx { kind, angle, dist, t: 0.0, ttl });
    }
}

/// Player commands raised by the UI, consumed by the simulation.
#[derive(Resource, Default)]
pub struct PilotCommands {
    pub fire_rail: bool,
    pub vent: bool,
    pub scram: bool,
    pub ecm: bool,
    pub eject: bool,
    pub crawl_to: Option<Section>,
    pub return_to_seat: bool,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Radio>()
            .init_resource::<Effects>()
            .init_resource::<PilotCommands>()
            .add_systems(
                Update,
                (
                    tick_mission,
                    spawn_waves,
                    power_and_heat,
                    pilot_actions,
                    weapons,
                    enemy_ai,
                    drones,
                    shells,
                    boarders,
                    check_outcome,
                )
                    .chain()
                    .run_if(mission_running),
            )
            .add_systems(Update, tick_effects);
    }
}

pub fn mission_running(mission: Option<Res<Mission>>) -> bool {
    mission.is_some_and(|m| m.running())
}

/// Simulation step, slightly clamped so a hitch does not teleport enemies.
fn dt(time: &Time) -> f32 {
    time.delta_secs().min(0.05)
}

fn tick_mission(time: Res<Time>, mut mission: ResMut<Mission>, mut radio: ResMut<Radio>, mut mech: ResMut<Mech>) {
    let dt = dt(&time);
    mission.time += dt;
    radio.time = mission.time;
    mech.sweep = (mech.sweep + dt * 2.2) % TAU;
}

struct Wave {
    at: f32,
    kinds: &'static [EnemyKind],
}

const WAVES: &[Wave] = &[
    Wave { at: 4.0, kinds: &[EnemyKind::Scout, EnemyKind::Scout] },
    Wave { at: 25.0, kinds: &[EnemyKind::Artillery] },
    Wave { at: 50.0, kinds: &[EnemyKind::Carrier, EnemyKind::Scout] },
    Wave { at: 75.0, kinds: &[EnemyKind::Scout, EnemyKind::Scout, EnemyKind::Scout] },
    Wave { at: 105.0, kinds: &[EnemyKind::Artillery, EnemyKind::Scout, EnemyKind::Scout] },
    Wave { at: 135.0, kinds: &[EnemyKind::Carrier, EnemyKind::Artillery] },
    Wave { at: 165.0, kinds: &[EnemyKind::Scout, EnemyKind::Scout, EnemyKind::Scout] },
    Wave { at: 195.0, kinds: &[EnemyKind::Artillery, EnemyKind::Carrier, EnemyKind::Scout] },
];

pub fn spawn_enemy(commands: &mut Commands, rng: &mut Rng, mission: &mut Mission, kind: EnemyKind, angle: f32, dist: f32) {
    let scale = 1.0 + 0.15 * (mission.number.saturating_sub(1)) as f32;
    let (hp, hold) = match kind {
        EnemyKind::Scout => (60.0, rng.range(150.0, 210.0)),
        EnemyKind::Artillery => (200.0, rng.range(680.0, 760.0)),
        EnemyKind::Carrier => (150.0, rng.range(320.0, 370.0)),
    };
    let serial = mission.serial;
    mission.serial += 1;
    commands.spawn(Enemy {
        kind,
        serial,
        hp: hp * scale,
        max_hp: hp * scale,
        angle,
        dist,
        hold_dist: hold,
        state: EnemyState::Approach,
        timer: rng.range(1.0, 3.0),
        orbit: if rng.chance(0.5) { 1.0 } else { -1.0 },
        stun: 0.0,
        drones: 4,
        hit_flash: 0.0,
        reveal: 0.0,
    });
}

fn spawn_waves(
    mut commands: Commands,
    mut mission: ResMut<Mission>,
    mut rng: ResMut<Rng>,
    mut radio: ResMut<Radio>,
    enemies: Query<&Enemy>,
) {
    let t = mission.time;
    let due = if mission.next_wave < WAVES.len() {
        (WAVES[mission.next_wave].at <= t).then(|| WAVES[mission.next_wave].kinds.to_vec())
    } else {
        // Endless pressure after the scripted waves, while the uplink is still down.
        let last = WAVES.last().unwrap().at;
        let k = mission.next_wave - WAVES.len() + 1;
        (t >= last + 30.0 * k as f32 && enemies.iter().count() < 7).then(|| {
            match rng.index(3) {
                0 => vec![EnemyKind::Scout, EnemyKind::Scout],
                1 => vec![EnemyKind::Artillery, EnemyKind::Scout],
                _ => vec![EnemyKind::Carrier],
            }
        })
    };
    let Some(mut kinds) = due else { return };
    if mission.number >= 2 && mission.next_wave % 2 == 1 {
        kinds.push(EnemyKind::Scout);
    }
    mission.next_wave += 1;
    let bearing = rng.range(0.0, TAU);
    for kind in kinds {
        let a = bearing + rng.range(-0.5, 0.5);
        let d = rng.range(950.0, 1100.0);
        spawn_enemy(&mut commands, &mut rng, &mut mission, kind, a, d);
        match kind {
            EnemyKind::Scout => {}
            EnemyKind::Artillery => radio.say(
                Voice::Control,
                format!("Bastion, seismic thump bearing {:03.0}. Something heavy is setting up out there.", bearing_deg(a)),
            ),
            EnemyKind::Carrier => radio.say(
                Voice::Control,
                format!("Hive signature bearing {:03.0}. Carrier class. Don't let the ticks latch on.", bearing_deg(a)),
            ),
        }
    }
}

pub fn bearing_deg(a: f32) -> f32 {
    let d = a.to_degrees().rem_euclid(360.0);
    if d >= 359.5 { 0.0 } else { d }
}

fn power_and_heat(
    time: Res<Time>,
    campaign: Res<Campaign>,
    mut mission: ResMut<Mission>,
    mut mech: ResMut<Mech>,
    mut radio: ResMut<Radio>,
    boarders: Query<&Boarder>,
    mut cmds: ResMut<PilotCommands>,
) {
    let dt = dt(&time);
    let m = &campaign.mods;
    let n_boarders = boarders.iter().count() as f32;

    let st = derive(&mech, m, n_boarders);
    let eff = st.eff;
    let parasitic = st.parasitic;
    mech.heat = (mech.heat + (st.heat_in - st.heat_out) * dt).clamp(0.0, HEAT_MAX);
    mech.heat_peak = mech.heat_peak.max(mech.heat);
    if mech.heat > 90.0 {
        // Lines cook when the core runs hot.
        mech.damage(Section::Coolant, 0.6 * dt);
    }

    // --- Shields -------------------------------------------------------------------
    mech.shield_hit_t += dt;
    if mech.shields_down_t > 0.0 {
        mech.shields_down_t -= dt;
        mech.shield = 0.0;
    } else {
        let regen = eff[Sys::Shields.idx()] * if mech.shield_hit_t > 1.5 { 0.5 } else { 0.15 };
        mech.shield = (mech.shield + regen * dt).min(st.shield_cap);
    }

    // --- Repair drones ---------------------------------------------------------------
    let rt = mech.repair_target;
    if mech.alloc[Sys::Repair.idx()] > 0.0 {
        let i = rt.idx();
        if rt == Section::Uplink {
            mech.hp[i] = (mech.hp[i] + eff[Sys::Repair.idx()] * UPLINK_RATE * m.repair_mult * dt).min(100.0);
        } else {
            mech.hp[i] = (mech.hp[i] + st.repair_rate * dt).min(mech.max_hp[i]);
        }
    }

    // --- Timers ----------------------------------------------------------------------
    let mm = &mut *mech;
    for t in [&mut mm.vent_t, &mut mm.vent_cd, &mut mm.scram_t, &mut mm.scram_cd, &mut mm.ecm_cd] {
        *t = (*t - dt).max(0.0);
    }
    mech.hit_flash = (mech.hit_flash - dt * 2.5).max(0.0);
    mech.shake = (mech.shake - dt * 3.0).max(0.0);

    // --- Emergency actions -------------------------------------------------------------
    if std::mem::take(&mut cmds.vent) && mech.vent_cd <= 0.0 {
        mech.vent_t = 2.0;
        mech.vent_cd = 25.0;
        mech.shields_down_t = 6.0;
        mech.damage(Section::Coolant, 4.0);
        radio.say(Voice::System, "COOLANT VENT OPEN. SHIELD EMITTERS OFFLINE 6s.");
    }
    if std::mem::take(&mut cmds.scram) && mech.scram_cd <= 0.0 {
        mech.scram_t = 6.0;
        mech.scram_cd = 30.0;
        radio.say(Voice::System, "SCRAM. CONTROL RODS DOWN. ALL BUSES DEAD FOR 6s.");
        radio.say(Voice::Pilot, "Come on, come on... stay cold for me.");
    }

    // --- Cockpit breach --------------------------------------------------------------
    if mech.frac(Section::Cockpit) < 0.25 && !mech.interlocks_failed {
        if mech.breach.is_none() {
            mech.breach = Some(BREACH_TIME);
            radio.say(Voice::System, "COCKPIT BREACH. LIFE SUPPORT ON RESERVE. REACTOR INTERLOCKS ON MANUAL.");
            radio.say(Voice::Chief, "Patch that canopy, kid! When the air goes, so do the reactor interlocks.");
        }
    } else if mech.frac(Section::Cockpit) >= 0.35 && mech.breach.is_some() {
        mech.breach = None;
        radio.say(Voice::System, "COCKPIT SEALED.");
    }
    if let Some(t) = mech.breach.as_mut() {
        *t -= dt;
        if *t <= 0.0 {
            mech.breach = None;
            mech.interlocks_failed = true;
            radio.say(Voice::System, "INTERLOCK FAILURE. REACTOR REGULATION LOST. CORE HEAT RISING.");
        }
    }

    // --- Reactor overload: the failure state -------------------------------------------
    if mech.heat >= HEAT_CRIT {
        if mech.overload.is_none() {
            mech.overload = Some(OVERLOAD_TIME);
            radio.say(Voice::System, "!! REACTOR OVERLOAD !! CONTAINMENT FAILURE IMMINENT.");
            radio.say(Voice::Chief, "Vent it, SCRAM it, or get out of there!");
        }
    } else if mech.heat < 92.0 && mech.overload.is_some() {
        mech.overload = None;
        radio.say(Voice::System, "Core temperature receding. Overload countdown aborted.");
        radio.say(Voice::Pilot, "...Breathe.");
    }
    if let Some(t) = mech.overload.as_mut() {
        *t -= dt;
    }
    if mech.heat > 80.0 && mission.warn_once("heat80") {
        radio.say(Voice::Chief, "Core's running hot. Ease off the rail or give me more cooling.");
    }
    if n_boarders > 0.0 && mission.warn_once("hack") {
        radio.say(Voice::System, format!("PARASITIC DRAW DETECTED: -{:.0} UNITS", parasitic));
    }

    mech.stats = st;
}

/// Everything the reactor bus and the damage state imply this instant.
pub fn derive(mech: &Mech, m: &Mods, n_boarders: f32) -> Derived {
    // --- Reactor output and the bus -------------------------------------------------
    let reactor_eff = 0.5 + 0.5 * mech.frac(Section::Reactor);
    let parasitic = n_boarders * 6.0 * m.hack_mult;
    let available = (m.reactor_output * reactor_eff - parasitic).max(0.0);
    let draw: f32 = mech.alloc.iter().sum();
    let ceiling = available * OVERDRIVE;
    let mut supply = if draw > ceiling { ceiling / draw } else { 1.0 };
    if mech.scram_t > 0.0 {
        supply = 0.0;
    }
    let mut eff = [0.0; 7];
    for s in Sys::ALL {
        eff[s.idx()] = mech.alloc[s.idx()] * supply;
    }
    let delivered = draw * supply;
    let overdrive = (delivered - available).max(0.0);

    // --- Derived performance --------------------------------------------------------
    let sens = eff[Sys::Sensors.idx()];
    let sensor_factor = if mech.hp[Section::Sensors.idx()] <= 0.0 { 0.25 } else { 0.35 + 0.65 * mech.frac(Section::Sensors) };
    let radar_range = ((220.0 + sens * 26.0 + m.sensor_range) * sensor_factor).max(120.0);
    let legs = (mech.eff(Section::LeftLeg) + mech.eff(Section::RightLeg)) * 0.5;
    let leg_penalty = if mech.hp[Section::LeftLeg.idx()] <= 0.0 || mech.hp[Section::RightLeg.idx()] <= 0.0 { 0.4 } else { 1.0 };
    let evasion = (eff[Sys::Locomotion.idx()] / 30.0 * m.loco_mult * legs * leg_penalty).clamp(0.0, 1.0);
    let cooling = eff[Sys::Cooling.idx()] * 0.5 * m.cooling_mult * mech.eff(Section::Coolant);
    let shield_cap = eff[Sys::Shields.idx()] * 4.0 * m.shield_mult * mech.eff(Section::ShieldGen);
    let rail_rate = eff[Sys::Railgun.idx()] / 40.0 * 0.3 * m.rail_charge_mult * mech.eff(Section::Railgun);
    let ac_dps = eff[Sys::Autocannon.idx()] * 0.7 * m.ac_mult * mech.eff(Section::Autocannon);
    let repair_rate = eff[Sys::Repair.idx()] * 0.16 * m.repair_mult;

    // --- Heat ----------------------------------------------------------------------
    let mut heat_in = (1.0 + delivered * 0.03) * m.base_heat_mult + overdrive * OVERDRIVE_HEAT;
    heat_in += (1.0 - mech.frac(Section::Reactor)) * 4.0;
    if mech.frac(Section::Coolant) < 0.35 {
        heat_in += 2.5;
    }
    if mech.interlocks_failed {
        heat_in += 3.0;
    }
    if mech.ac_firing {
        heat_in += eff[Sys::Autocannon.idx()] * 0.14;
    }
    let mut heat_out = cooling;
    if mech.scram_t > 0.0 {
        heat_out += 10.0;
        heat_in = 0.5;
    }
    if mech.vent_t > 0.0 {
        heat_out += 20.0;
    }

    Derived {
        available,
        parasitic,
        draw,
        supply,
        eff,
        radar_range,
        id_range: radar_range * 0.65,
        evasion,
        cooling,
        heat_in,
        heat_out,
        shield_cap,
        rail_rate,
        ac_dps,
        repair_rate,
    }
}

fn pilot_actions(
    time: Res<Time>,
    mut mech: ResMut<Mech>,
    mut cmds: ResMut<PilotCommands>,
    mut radio: ResMut<Radio>,
    mut commands: Commands,
    boarders: Query<(Entity, &Boarder)>,
) {
    let dt = dt(&time);
    if let Some(to) = cmds.crawl_to.take()
        && mech.pilot.in_seat()
    {
        let dur = crawl_time(to);
        mech.pilot = Pilot::Crawling { to, t: 0.0, dur };
        radio.say(Voice::Pilot, format!("Leaving the chair. Crawling to the {}.", to.name().to_lowercase()));
    }
    let want_return = std::mem::take(&mut cmds.return_to_seat);
    mech.pilot = match mech.pilot {
        Pilot::Seat => Pilot::Seat,
        Pilot::Crawling { to, t, dur } => {
            if t + dt >= dur {
                Pilot::Working { at: to, t: 0.0 }
            } else {
                Pilot::Crawling { to, t: t + dt, dur }
            }
        }
        Pilot::Working { at, t } => {
            if want_return {
                Pilot::Returning { from: at, t: 0.0, dur: crawl_time(at) }
            } else {
                let t = t + dt;
                let here: Vec<Entity> = boarders.iter().filter(|(_, b)| b.section == at).map(|(e, _)| e).collect();
                if !here.is_empty() {
                    if t >= 2.2 {
                        commands.entity(here[0]).despawn();
                        radio.say(Voice::Pilot, "Got one. Pried it off with the wrench.");
                        Pilot::Working { at, t: 0.0 }
                    } else {
                        Pilot::Working { at, t }
                    }
                } else {
                    // Hands-on repair needs no reactor power.
                    let i = at.idx();
                    if at == Section::Uplink {
                        mech.hp[i] = (mech.hp[i] + 0.35 * dt).min(100.0);
                    } else {
                        mech.hp[i] = (mech.hp[i] + 5.0 * dt).min(mech.max_hp[i]);
                    }
                    Pilot::Working { at, t }
                }
            }
        }
        Pilot::Returning { from, t, dur } => {
            if t + dt >= dur {
                radio.say(Voice::Pilot, "Back in the chair.");
                Pilot::Seat
            } else {
                Pilot::Returning { from, t: t + dt, dur }
            }
        }
    };
}

fn weapons(
    time: Res<Time>,
    campaign: Res<Campaign>,
    mut mech: ResMut<Mech>,
    mut cmds: ResMut<PilotCommands>,
    mut rng: ResMut<Rng>,
    mut fx: ResMut<Effects>,
    mut radio: ResMut<Radio>,
    mut commands: Commands,
    mut enemies: Query<(Entity, &mut Enemy)>,
    mut drones_q: Query<(Entity, &mut Drone)>,
    mut boarders: Query<(Entity, &Boarder)>,
) {
    let dt = dt(&time);
    let m = &campaign.mods;
    let st = mech.stats;
    let seated = mech.pilot.in_seat();

    // Drop dead/invisible targets.
    if let Some(t) = mech.target
        && enemies.get(t).is_err()
    {
        mech.target = None;
    }

    // --- Railgun -------------------------------------------------------------------------
    mech.rail_charge = (mech.rail_charge + st.rail_rate * dt).min(1.0);
    let fire = std::mem::take(&mut cmds.fire_rail) || (mech.rail_auto && seated);
    if fire && seated && mech.rail_charge >= 1.0 {
        let tgt = mech.target.and_then(|t| enemies.get(t).ok()).filter(|(_, e)| e.dist <= st.radar_range).map(|(e, _)| e);
        if let Some(te) = tgt {
            let (_, mut e) = enemies.get_mut(te).unwrap();
            mech.rail_charge = 0.0;
            mech.heat += 22.0 + m.rail_heat;
            mech.shake = 1.0;
            let mut p = 0.92 * (0.5 + 0.5 * mech.eff(Section::Sensors));
            if e.kind == EnemyKind::Scout {
                p -= 0.15;
            }
            fx.add(FxKind::Rail, e.angle, e.dist, 0.45);
            if rng.chance(p) {
                e.hp -= 75.0 * m.rail_damage_mult;
                e.hit_flash = 0.4;
                fx.add(FxKind::Explosion, e.angle, e.dist, 0.7);
            } else {
                radio.say(Voice::Pilot, "Missed! Recharging...");
            }
        }
    }

    // --- Autocannon ----------------------------------------------------------------------
    mech.ac_firing = false;
    if seated && st.ac_dps > 0.0 && mech.weapons_free {
        mech.ac_burst_t += dt;
        // Point defence: incoming boarding drones first.
        let mut drone_target: Option<Entity> = None;
        if mech.point_defence {
            let mut best = AC_RANGE;
            for (e, d) in drones_q.iter() {
                if d.dist < best {
                    best = d.dist;
                    drone_target = Some(e);
                }
            }
        }
        if let Some(de) = drone_target {
            let (_, mut d) = drones_q.get_mut(de).unwrap();
            d.hp -= st.ac_dps * 1.4 * dt;
            mech.ac_firing = true;
            if mech.ac_burst_t > 0.12 {
                fx.add(FxKind::Tracer, d.angle, d.dist, 0.15);
                mech.ac_burst_t = 0.0;
            }
            if d.hp <= 0.0 {
                fx.add(FxKind::Explosion, d.angle, d.dist, 0.35);
                commands.entity(de).despawn();
            }
        } else {
            let chosen = mech
                .target
                .and_then(|t| enemies.get(t).ok())
                .filter(|(_, e)| e.dist <= AC_RANGE)
                .map(|(e, _)| e)
                .or_else(|| {
                    enemies
                        .iter()
                        .filter(|(_, e)| e.dist <= AC_RANGE.min(st.radar_range))
                        .min_by(|a, b| a.1.dist.total_cmp(&b.1.dist))
                        .map(|(e, _)| e)
                });
            if let Some(te) = chosen {
                let (_, mut e) = enemies.get_mut(te).unwrap();
                let falloff = 1.0 - (e.dist / AC_RANGE) * 0.5;
                e.hp -= st.ac_dps * falloff * dt;
                e.hit_flash = e.hit_flash.max(0.08);
                mech.ac_firing = true;
                if mech.ac_burst_t > 0.12 {
                    fx.add(FxKind::Tracer, e.angle + rng.range(-0.02, 0.02), e.dist, 0.15);
                    mech.ac_burst_t = 0.0;
                }
            }
        }
    }

    // --- Electronic warfare --------------------------------------------------------------
    if std::mem::take(&mut cmds.ecm) && seated && mech.ecm_cd <= 0.0 && st.eff[Sys::Sensors.idx()] >= 5.0 {
        mech.ecm_cd = 28.0;
        mech.heat += 12.0;
        fx.add(FxKind::Ecm, 0.0, 0.0, 1.0);
        let mut fried = 0;
        for (e, d) in drones_q.iter() {
            if d.dist < 300.0 {
                fx.add(FxKind::Explosion, d.angle, d.dist, 0.35);
                commands.entity(e).despawn();
                fried += 1;
            }
        }
        for (e, _) in boarders.iter_mut() {
            if rng.chance(0.6) {
                commands.entity(e).despawn();
                fried += 1;
            }
        }
        for (_, mut e) in enemies.iter_mut() {
            if e.dist < 400.0 {
                e.stun = 3.5;
            }
        }
        radio.say(Voice::System, format!("ECM BURST. {fried} hostile drone(s) fried. Nearby contacts jammed."));
    }

    // --- Kills ----------------------------------------------------------------------------
    for (e, en) in enemies.iter() {
        if en.hp <= 0.0 {
            fx.add(FxKind::Explosion, en.angle, en.dist, 1.1);
            commands.entity(e).despawn();
            mech.kills += 1;
            if mech.target == Some(e) {
                mech.target = None;
            }
            let line = match en.kind {
                EnemyKind::Scout => "Scout down.",
                EnemyKind::Artillery => "Crab's cracked open. Artillery silenced.",
                EnemyKind::Carrier => "Carrier destroyed. No more ticks from that one.",
            };
            radio.say(Voice::Pilot, line);
        }
    }
}

fn enemy_ai(
    time: Res<Time>,
    mut mech: ResMut<Mech>,
    mut mission: ResMut<Mission>,
    mut rng: ResMut<Rng>,
    mut fx: ResMut<Effects>,
    mut radio: ResMut<Radio>,
    mut commands: Commands,
    mut enemies: Query<&mut Enemy>,
) {
    let dt = dt(&time);
    let evasion = mech.stats.evasion;
    for mut e in enemies.iter_mut() {
        e.hit_flash = (e.hit_flash - dt).max(0.0);
        e.reveal = (e.reveal - dt).max(0.0);
        if e.stun > 0.0 {
            e.stun -= dt;
            continue;
        }
        e.timer -= dt;
        match e.kind {
            EnemyKind::Scout => {
                if e.dist > e.hold_dist {
                    e.dist -= 40.0 * dt;
                    e.angle += 0.12 * e.orbit * dt;
                } else {
                    e.angle += 0.22 * e.orbit * dt;
                    e.dist += (rng.range(-1.0, 1.0)) * 20.0 * dt;
                }
                // Jackals open fire while still closing.
                if e.dist < 400.0 && e.timer <= 0.0 {
                    {
                        e.timer = rng.range(1.6, 2.4);
                        fx.add(FxKind::Muzzle, e.angle, e.dist, 0.2);
                        let close = if e.dist < 250.0 { 1.0 } else { 0.7 };
                        if rng.chance(0.65 * close * (1.0 - 0.6 * evasion)) {
                            const TARGETS: [Section; 7] = [
                                Section::Sensors,
                                Section::Cockpit,
                                Section::LeftLeg,
                                Section::RightLeg,
                                Section::Autocannon,
                                Section::Railgun,
                                Section::Coolant,
                            ];
                            let s = TARGETS[rng.index(TARGETS.len())];
                            hit_mech(&mut mech, &mut mission, s, rng.range(5.0, 8.0));
                        }
                    }
                }
            }
            EnemyKind::Artillery => match e.state {
                EnemyState::Approach => {
                    e.dist -= 14.0 * dt;
                    if e.dist <= e.hold_dist {
                        e.state = EnemyState::Deploying;
                        e.timer = 3.0;
                    }
                }
                EnemyState::Deploying => {
                    if e.timer <= 0.0 {
                        e.state = EnemyState::Engaged;
                        e.timer = 0.5;
                        radio.say(Voice::Enemy, format!("[INTERCEPT] {}-{:02}: braced. Ranging on the walker.", e.kind.callsign(), e.serial));
                    }
                }
                EnemyState::Engaged => {
                    if e.timer <= 0.0 {
                        e.timer = rng.range(8.0, 10.0);
                        e.reveal = 3.0;
                        fx.add(FxKind::Muzzle, e.angle, e.dist, 0.4);
                        const TARGETS: [(Section, u32); 7] = [
                            (Section::Reactor, 3),
                            (Section::LeftLeg, 2),
                            (Section::RightLeg, 2),
                            (Section::Cockpit, 2),
                            (Section::Uplink, 2),
                            (Section::ShieldGen, 1),
                            (Section::Coolant, 2),
                        ];
                        let total: u32 = TARGETS.iter().map(|t| t.1).sum();
                        let mut roll = rng.index(total as usize) as u32;
                        let mut target = Section::Reactor;
                        for (s, w) in TARGETS {
                            if roll < w {
                                target = s;
                                break;
                            }
                            roll -= w;
                        }
                        commands.spawn(Shell { target, t: 0.0, flight: 4.5, angle: e.angle });
                    }
                }
            },
            EnemyKind::Carrier => {
                if e.dist > e.hold_dist {
                    e.dist -= 24.0 * dt;
                } else {
                    e.angle += 0.03 * e.orbit * dt;
                    if e.timer <= 0.0 && e.drones > 0 {
                        e.timer = rng.range(4.5, 6.0);
                        e.drones -= 1;
                        commands.spawn(Drone { angle: e.angle, dist: e.dist - 10.0, hp: 14.0 });
                        if mission.warn_once("first_drone") {
                            radio.say(Voice::Chief, "Boarding drones! Point defence on, or they'll chew through the hull.");
                        }
                    }
                }
            }
        }
        e.angle = e.angle.rem_euclid(TAU);
    }
}

fn hit_mech(mech: &mut Mech, mission: &mut Mission, s: Section, dmg: f32) {
    let absorbed = dmg.min(mech.shield);
    mech.shield -= absorbed;
    mech.heat += absorbed * 0.35;
    mech.shield_hit_t = 0.0;
    let rest = dmg - absorbed;
    if rest > 0.0 {
        mech.damage(s, rest);
        mission.damage_taken[s.idx()] += rest;
        mech.hit_flash = 1.0;
    }
}

fn drones(
    time: Res<Time>,
    mut rng: ResMut<Rng>,
    mut radio: ResMut<Radio>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Drone)>,
) {
    let dt = dt(&time);
    for (e, mut d) in q.iter_mut() {
        d.dist -= 70.0 * dt;
        d.angle += (d.dist * 0.05).sin() * 0.2 * dt;
        if d.dist <= 8.0 {
            commands.entity(e).despawn();
            const SPOTS: [Section; 8] = [
                Section::Sensors,
                Section::Coolant,
                Section::Railgun,
                Section::Autocannon,
                Section::ShieldGen,
                Section::Reactor,
                Section::LeftLeg,
                Section::RightLeg,
            ];
            let s = SPOTS[rng.index(SPOTS.len())];
            commands.spawn(Boarder { section: s, age: 0.0 });
            radio.say(Voice::System, format!("HULL CONTACT: boarding drone latched onto {}.", s.name()));
        }
    }
}

fn shells(
    time: Res<Time>,
    mut mech: ResMut<Mech>,
    mut mission: ResMut<Mission>,
    mut rng: ResMut<Rng>,
    mut fx: ResMut<Effects>,
    mut radio: ResMut<Radio>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Shell)>,
) {
    let dt = dt(&time);
    for (e, mut s) in q.iter_mut() {
        s.t += dt;
        if s.t >= s.flight {
            commands.entity(e).despawn();
            let p = 0.85 - 0.55 * mech.stats.evasion;
            if rng.chance(p) {
                let dmg = if s.target == Section::Uplink { 8.0 } else { rng.range(16.0, 24.0) };
                hit_mech(&mut mech, &mut mission, s.target, dmg);
                let splash = Section::ALL[rng.index(9)];
                hit_mech(&mut mech, &mut mission, splash, 5.0);
                mech.shake = 1.4;
                fx.add(FxKind::ShellImpact, s.angle, 30.0, 0.8);
                if s.target == Section::Uplink {
                    radio.say(Voice::Control, "Uplink took a hit! Restore progress lost.");
                }
            } else {
                fx.add(FxKind::ShellImpact, s.angle + 0.4, 120.0, 0.8);
                if mission.warn_once("dodge") {
                    radio.say(Voice::Pilot, "Stepped clear of that one. Keep the legs moving.");
                }
            }
        }
    }
}

fn boarders(time: Res<Time>, campaign: Res<Campaign>, mut mech: ResMut<Mech>, mut mission: ResMut<Mission>, mut q: Query<&mut Boarder>) {
    let dt = dt(&time);
    for mut b in q.iter_mut() {
        b.age += dt;
        let dmg = 1.2 * campaign.mods.hack_mult * dt;
        mech.damage(b.section, dmg);
        mission.damage_taken[b.section.idx()] += dmg;
    }
}

fn check_outcome(mut mech: ResMut<Mech>, mut mission: ResMut<Mission>, mut radio: ResMut<Radio>, mut cmds: ResMut<PilotCommands>) {
    if mech.uplink() >= 100.0 {
        mission.outcome = Some(Outcome::Victory);
        radio.say(Voice::Control, "Uplink green! The city grid is talking to us again. Bring her home, Bastion.");
    } else if mech.overload.is_some_and(|t| t <= 0.0) {
        mission.outcome = Some(Outcome::Meltdown);
        mech.heat = HEAT_MAX;
        radio.say(Voice::System, "CONTAINMENT LOST.");
    } else if std::mem::take(&mut cmds.eject) {
        mission.outcome = Some(Outcome::Ejected);
        radio.say(Voice::Pilot, "Punching out!");
    }
}

fn tick_effects(time: Res<Time>, mission: Option<Res<Mission>>, mut fx: ResMut<Effects>) {
    if mission.is_some_and(|m| m.paused) {
        return;
    }
    let dt = time.delta_secs();
    for f in fx.list.iter_mut() {
        f.t += dt;
    }
    fx.list.retain(|f| f.t < f.ttl);
}

/// Angle difference wrapped to [-PI, PI].
pub fn angle_diff(a: f32, b: f32) -> f32 {
    (a - b + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    /// A crude pilot: keeps repair on the uplink, snipes what it can see, vents
    /// when hot and pushes sensors up once artillery starts landing.
    fn autopilot(
        mut mech: ResMut<Mech>,
        mut cmds: ResMut<PilotCommands>,
        enemies: Query<(Entity, &Enemy)>,
        boarders: Query<&Boarder>,
    ) {
        let range = mech.stats.radar_range;
        let pick = enemies
            .iter()
            .filter(|(_, e)| e.dist <= range)
            .max_by_key(|(_, e)| match e.kind {
                EnemyKind::Artillery => 3,
                EnemyKind::Carrier => 2,
                EnemyKind::Scout => 1,
            })
            .map(|(e, _)| e);
        mech.target = pick;
        mech.alloc = if mech.heat > 60.0 { [5.0, 20.0, 10.0, 10.0, 10.0, 15.0, 30.0] } else { [5.0, 20.0, 20.0, 10.0, 10.0, 20.0, 15.0] };
        if mech.heat < 70.0 && mech.rail_charge >= 1.0 {
            cmds.fire_rail = true;
        }
        if mech.heat > 88.0 {
            cmds.vent = true;
        }
        if mech.overload.is_some() {
            cmds.scram = true;
        }
        let worst = Section::ALL[..9].iter().copied().min_by(|a, b| mech.frac(*a).total_cmp(&mech.frac(*b))).unwrap();
        mech.repair_target = if mech.frac(worst) < 0.3 { worst } else { Section::Uplink };
        if let Some(b) = boarders.iter().next()
            && mech.pilot.in_seat()
            && boarders.iter().count() >= 3
        {
            cmds.crawl_to = Some(b.section);
        }
        if let Pilot::Working { at, .. } = mech.pilot
            && boarders.iter().all(|b| b.section != at)
        {
            cmds.return_to_seat = true;
        }
    }

    fn run(seed: u64, with_pilot: bool) -> (Option<Outcome>, f32, f32, f32) {
        let r = run_full(seed, with_pilot);
        (r.0, r.1, r.2, r.3)
    }

    fn run_full(seed: u64, with_pilot: bool) -> (Option<Outcome>, f32, f32, f32, u32, f32, f32) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(50)))
            .add_plugins(SimPlugin)
            .insert_resource(Rng::new(seed))
            .init_resource::<Campaign>();
        let campaign = Campaign::default();
        app.insert_resource(Mech::new(&campaign));
        let mut mission = Mission::new(1);
        mission.briefing = false;
        app.insert_resource(mission);
        if with_pilot {
            app.add_systems(Update, autopilot.before(tick_mission));
        }
        for i in 0..(20 * 600) {
            app.update();
            if std::env::var("HZ_DEBUG").is_ok() && i % 100 == 0 {
                let mech = app.world().resource::<Mech>();
                let st = mech.stats;
                eprintln!(
                    "t={:5.0} heat={:5.1} in={:4.1} out={:4.1} cool={:3.2} rct={:3.2} cpt={:3.2} lock={} scram_cd={:2.0} vent_cd={:2.0} shield={:3.0} sup={:.2} par={}",
                    app.world().resource::<Mission>().time, mech.heat, st.heat_in, st.heat_out, mech.frac(Section::Coolant), mech.frac(Section::Reactor), mech.frac(Section::Cockpit), mech.interlocks_failed, mech.scram_cd, mech.vent_cd, mech.shield, st.supply, st.parasitic
                );
            }
            if app.world().resource::<Mission>().outcome.is_some() {
                break;
            }
        }
        let m = app.world().resource::<Mission>();
        let mech = app.world().resource::<Mech>();
        let worst = Section::ALL[..9].iter().map(|s| mech.frac(*s)).fold(1.0, f32::min);
        (m.outcome, m.time, mech.uplink(), mech.heat_peak, mech.kills, m.damage_taken.iter().sum(), worst)
    }

    #[test]
    fn idle_pilot_still_produces_a_result() {
        let (outcome, time, uplink, _) = run(3, false);
        assert!(outcome.is_some() || uplink > 0.0, "nothing happened in {time}s");
    }

    #[test]
    fn autopilot_mission_lands_in_target_length() {
        for seed in [1, 2, 3, 4] {
            let (outcome, time, uplink, peak, kills, dmg, worst) = run_full(seed, true);
            println!("seed {seed}: {outcome:?} after {time:.0}s, uplink {uplink:.0}%, peak heat {peak:.0}, kills {kills}, damage {dmg:.0}, worst section {worst:.2}");
            assert!(outcome.is_some(), "seed {seed}: mission did not resolve in 10 minutes");
            assert!((150.0..=480.0).contains(&time), "seed {seed}: mission took {time:.0}s");
        }
    }

    #[test]
    fn overdrive_costs_heat() {
        let campaign = Campaign::default();
        let mut mech = Mech::new(&campaign);
        let calm = derive(&mech, &campaign.mods, 0.0);
        for s in Sys::ALL {
            mech.alloc[s.idx()] = s.cap();
        }
        let hot = derive(&mech, &campaign.mods, 0.0);
        assert!(hot.supply < 1.0, "205 units cannot all be delivered");
        assert!(hot.heat_in > calm.heat_in + 5.0);
    }
}
