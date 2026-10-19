//! Persistent state between operations: scars, salvaged parts, and how much of the
//! original walking fortress is left.

use bevy::prelude::*;

use crate::rng::Rng;
use crate::sim::{Mission, Section};

/// Who built the part currently bolted into a section. Replacing a part changes the
/// faceplate (name + accent colour) of the matching cockpit station.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Maker {
    Bastion,
    KesslerVolt,
    Ostrava,
    Thalassa,
    CivicTransit,
    Jackal,
    Hive,
    Crab,
}

impl Maker {
    pub fn name(self) -> &'static str {
        match self {
            Maker::Bastion => "BASTION WORKS",
            Maker::KesslerVolt => "KESSLER-VOLT",
            Maker::Ostrava => "OSTRAVA HEAVY",
            Maker::Thalassa => "THALASSA NAVAL",
            Maker::CivicTransit => "CIVIC TRANSIT",
            Maker::Jackal => "JACKAL (ENEMY)",
            Maker::Hive => "HIVE (GRAFT)",
            Maker::Crab => "CRAB (ENEMY)",
        }
    }

    /// Accent colour as RGB.
    pub fn accent(self) -> [u8; 3] {
        match self {
            Maker::Bastion => [232, 168, 56],
            Maker::KesslerVolt => [96, 200, 255],
            Maker::Ostrava => [214, 96, 64],
            Maker::Thalassa => [120, 220, 190],
            Maker::CivicTransit => [240, 220, 120],
            Maker::Jackal => [190, 210, 90],
            Maker::Hive => [190, 110, 240],
            Maker::Crab => [200, 140, 110],
        }
    }

    pub fn original(self) -> bool {
        self == Maker::Bastion
    }
}

#[derive(Clone, Debug)]
pub struct Mods {
    pub reactor_output: f32,
    pub rail_charge_mult: f32,
    pub rail_heat: f32,
    pub rail_damage_mult: f32,
    pub sensor_range: f32,
    pub cooling_mult: f32,
    pub shield_mult: f32,
    pub repair_mult: f32,
    pub loco_mult: f32,
    pub ac_mult: f32,
    pub hack_mult: f32,
    pub base_heat_mult: f32,
}

impl Default for Mods {
    fn default() -> Self {
        Self {
            reactor_output: 100.0,
            rail_charge_mult: 1.0,
            rail_heat: 0.0,
            rail_damage_mult: 1.0,
            sensor_range: 0.0,
            cooling_mult: 1.0,
            shield_mult: 1.0,
            repair_mult: 1.0,
            loco_mult: 1.0,
            ac_mult: 1.0,
            hack_mult: 1.0,
            base_heat_mult: 1.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PartId {
    CapacitorBank,
    OstravaActuator,
    JackalDish,
    NavyRadiator,
    HiveReactor,
    CrabDampers,
    TramBattery,
    OriginalSpares,
    FuelRods,
}

pub struct PartInfo {
    pub id: PartId,
    pub name: &'static str,
    pub maker: Maker,
    pub section: Section,
    pub pros: &'static str,
    pub cons: &'static str,
    pub flavor: &'static str,
    /// Pixel-art icon key, see `art::salvage_icon`.
    pub icon: usize,
}

pub const PARTS: &[PartInfo] = &[
    PartInfo {
        id: PartId::CapacitorBank,
        name: "KV-9 Capacitor Bank",
        maker: Maker::KesslerVolt,
        section: Section::Railgun,
        pros: "+35% railgun charge rate",
        cons: "+6 heat per shot",
        flavor: "Pulled from a substation. Still hums.",
        icon: 0,
    },
    PartInfo {
        id: PartId::OstravaActuator,
        name: "Ostrava Leg Actuator",
        maker: Maker::Ostrava,
        section: Section::LeftLeg,
        pros: "Rebuilds weaker leg to 110 max, +25% locomotion",
        cons: "Different gait: left/right legs mismatched",
        flavor: "Mining-walker knee. Twice the torque, half the grace.",
        icon: 1,
    },
    PartInfo {
        id: PartId::JackalDish,
        name: "Jackal Sensor Dish",
        maker: Maker::Jackal,
        section: Section::Sensors,
        pros: "+140m radar range, square-scope readout",
        cons: "Enemy firmware. Sensor head max HP -15",
        flavor: "Peeled off a dead scout. It still pings its old pack.",
        icon: 2,
    },
    PartInfo {
        id: PartId::NavyRadiator,
        name: "Navy Surplus Radiator",
        maker: Maker::Thalassa,
        section: Section::Coolant,
        pros: "+30% cooling, coolant lines max 120",
        cons: "Salt-crusted. Locomotion -10%",
        flavor: "Built to cool a destroyer's turbines.",
        icon: 3,
    },
    PartInfo {
        id: PartId::HiveReactor,
        name: "Hive-Grafted Reactor Sleeve",
        maker: Maker::Hive,
        section: Section::Reactor,
        pros: "+15 reactor output, +40% repair speed",
        cons: "Boarders feel at home: double parasitic drain",
        flavor: "It grew into the reactor housing overnight.",
        icon: 4,
    },
    PartInfo {
        id: PartId::CrabDampers,
        name: "Crab Recoil Dampers",
        maker: Maker::Crab,
        section: Section::Autocannon,
        pros: "+30% autocannon damage",
        cons: "Autocannon mount max HP -10",
        flavor: "Artillery shocks, cut down with a plasma torch.",
        icon: 5,
    },
    PartInfo {
        id: PartId::TramBattery,
        name: "Civic Tram Battery",
        maker: Maker::CivicTransit,
        section: Section::ShieldGen,
        pros: "+30% shield capacity",
        cons: "+10% base core heat",
        flavor: "Line 4 to the harbour. Nobody rides it anymore.",
        icon: 6,
    },
    PartInfo {
        id: PartId::OriginalSpares,
        name: "Bastion Original Spares",
        maker: Maker::Bastion,
        section: Section::Cockpit,
        pros: "Restores all scars on two most-worn sections",
        cons: "No upgrade. Just the machine as it was.",
        flavor: "A sealed crate stamped 1987.",
        icon: 7,
    },
    PartInfo {
        id: PartId::FuelRods,
        name: "Overclocked Fuel Rods",
        maker: Maker::KesslerVolt,
        section: Section::Reactor,
        pros: "+20 reactor output",
        cons: "+25% base core heat",
        flavor: "Enrichment unknown. Geiger says 'yes'.",
        icon: 8,
    },
];

pub fn part(id: PartId) -> &'static PartInfo {
    PARTS.iter().find(|p| p.id == id).unwrap()
}

#[derive(Clone, Debug)]
pub struct MissionReport {
    pub number: u32,
    pub time: f32,
    pub kills: u32,
    pub heat_peak: f32,
    pub scars: Vec<(Section, f32)>,
}

#[derive(Resource, Clone)]
pub struct Campaign {
    pub mission: u32,
    pub max_hp: [f32; 10],
    pub makers: [Maker; 10],
    pub mods: Mods,
    pub installed: Vec<PartId>,
    pub originality: f32,
    pub salvage: Vec<PartId>,
    pub last_report: Option<MissionReport>,
}

impl Default for Campaign {
    fn default() -> Self {
        Self {
            mission: 1,
            max_hp: [100.0; 10],
            makers: [Maker::Bastion; 10],
            mods: Mods::default(),
            installed: Vec::new(),
            originality: 100.0,
            salvage: Vec::new(),
            last_report: None,
        }
    }
}

impl Campaign {
    /// Permanent wear: every mission leaves scars proportional to damage taken.
    pub fn apply_scars(&mut self, mission: &Mission, kills: u32, heat_peak: f32) {
        let mut scars = Vec::new();
        for s in Section::ALL {
            if s == Section::Uplink {
                continue;
            }
            let taken = mission.damage_taken[s.idx()];
            let scar = (taken * 0.18).min(25.0).floor();
            if scar >= 1.0 {
                let i = s.idx();
                self.max_hp[i] = (self.max_hp[i] - scar).max(40.0);
                scars.push((s, scar));
            }
        }
        self.last_report = Some(MissionReport { number: mission.number, time: mission.time, kills, heat_peak, scars });
    }

    pub fn roll_salvage(&mut self, rng: &mut Rng) {
        let mut pool: Vec<PartId> = PARTS.iter().map(|p| p.id).filter(|id| !self.installed.contains(id) || *id == PartId::OriginalSpares).collect();
        self.salvage.clear();
        while self.salvage.len() < 5 && !pool.is_empty() {
            let i = rng.index(pool.len());
            self.salvage.push(pool.remove(i));
        }
    }

    pub fn install(&mut self, id: PartId) {
        let info = part(id);
        let m = &mut self.mods;
        match id {
            PartId::CapacitorBank => {
                m.rail_charge_mult *= 1.35;
                m.rail_heat += 6.0;
            }
            PartId::OstravaActuator => {
                let l = Section::LeftLeg.idx();
                let r = Section::RightLeg.idx();
                let i = if self.max_hp[l] <= self.max_hp[r] { l } else { r };
                self.max_hp[i] = 110.0;
                self.makers[i] = Maker::Ostrava;
                m.loco_mult *= 1.25;
            }
            PartId::JackalDish => {
                m.sensor_range += 140.0;
                let i = Section::Sensors.idx();
                self.max_hp[i] = (self.max_hp[i] - 15.0).max(40.0);
            }
            PartId::NavyRadiator => {
                m.cooling_mult *= 1.3;
                self.max_hp[Section::Coolant.idx()] = 120.0;
                m.loco_mult *= 0.9;
            }
            PartId::HiveReactor => {
                m.reactor_output += 15.0;
                m.repair_mult *= 1.4;
                m.hack_mult *= 2.0;
            }
            PartId::CrabDampers => {
                m.ac_mult *= 1.3;
                let i = Section::Autocannon.idx();
                self.max_hp[i] = (self.max_hp[i] - 10.0).max(40.0);
            }
            PartId::TramBattery => {
                m.shield_mult *= 1.3;
                m.base_heat_mult *= 1.1;
            }
            PartId::OriginalSpares => {
                let mut worn: Vec<Section> = Section::ALL.iter().copied().filter(|s| *s != Section::Uplink).collect();
                worn.sort_by(|a, b| self.max_hp[a.idx()].total_cmp(&self.max_hp[b.idx()]));
                for s in worn.iter().take(2) {
                    self.max_hp[s.idx()] = self.max_hp[s.idx()].max(100.0);
                    self.makers[s.idx()] = Maker::Bastion;
                }
                self.originality = (self.originality + 6.0).min(100.0);
            }
            PartId::FuelRods => {
                m.reactor_output += 20.0;
                m.base_heat_mult *= 1.25;
            }
        }
        if id != PartId::OstravaActuator && id != PartId::OriginalSpares {
            self.makers[info.section.idx()] = info.maker;
        }
        if !info.maker.original() {
            self.originality = (self.originality - 11.0).max(0.0);
        }
        self.installed.push(id);
    }

    /// The machine's identity, as the hangar describes it.
    pub fn identity(&self) -> &'static str {
        match self.originality as u32 {
            90..=100 => "WF-7 BASTION, as she left the factory",
            70..=89 => "BASTION, patched with salvage",
            45..=69 => "Half Bastion, half scrapyard",
            20..=44 => "Something wearing Bastion's skin",
            _ => "Unrecognisable. The city calls it 'the Chimera'",
        }
    }
}
