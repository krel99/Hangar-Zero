## Shared vocabulary of the simulation: sections, buses, cameras and the constants
## every other sim script leans on. Pure data, no state.
class_name Rules
extends RefCounted

enum Section { REACTOR, COCKPIT, SENSORS, COOLANT, LEFT_LEG, RIGHT_LEG, RAILGUN, AUTOCANNON, SHIELD_GEN, UPLINK }
enum Sys { LOCO, SENS, RAIL, AC, SHLD, REPR, COOL }
enum Kind { SCOUT, ARTILLERY, CARRIER }
enum EState { APPROACH, DEPLOYING, ENGAGED }
enum Voice { CONTROL, CHIEF, PILOT, ENEMY, SYSTEM }
enum Fx { RAIL, TRACER, EXPLOSION, MUZZLE, SHELL_IMPACT, ECM }
enum Outcome { NONE, VICTORY, MELTDOWN, EJECTED }
enum PilotState { SEAT, CRAWLING, WORKING, RETURNING }

const SECTIONS := 10
const SYSTEMS := 7

const SECTION_NAME := ["REACTOR", "COCKPIT", "SENSOR ARRAY", "COOLANT LINES", "LEFT LEG", "RIGHT LEG", "RAILGUN MOUNT", "AUTOCANNON MOUNT", "SHIELD EMITTERS", "CITY UPLINK"]
const SECTION_SHORT := ["RCT", "CPT", "SNS", "CLT", "L-LEG", "R-LEG", "RAIL", "A/C", "SHLD", "UPLNK"]
## Interior deck (0 = top of the mast, 4 = hips); drives crawl times.
const SECTION_DECK := [3, 1, 0, 3, 4, 4, 2, 2, 2, 0]

const SYS_NAME := ["LOCO", "SENS", "RAIL", "A/C", "SHLD", "REPR", "COOL"]
const SYS_BLURB := ["dodge shells and scouts", "radar range, tighter blips", "capacitor charge", "point defence, scouts", "absorbs hits, costs heat", "drones fix one section", "pulls heat from the core"]
## Reactor draw of each wired system. Set in the hangar; fixed in battle.
const MATRIX_COST := [15.0, 15.0, 20.0, 15.0, 15.0, 20.0, 15.0]
## Op 1 loadout: everything but locomotion (the left leg is scarred anyway).
const DEFAULT_MATRIX := [false, true, true, true, true, true, true]

const KIND_NAME := ["JACKAL SCOUT", "MORTAR CRAB", "TICK CARRIER"]
const KIND_CALLSIGN := ["JKL", "CRB", "TCK"]

## Four fixed hull cameras, one per quadrant.
const CAMERAS := 4
const CAMERA_HP := 60.0
## Below this a camera has no usable picture.
const CAMERA_MIN := 12.0
const CAMERA_FOV := PI / 2.0
const CAMERA_NAMES := ["FWD", "RIGHT", "AFT", "LEFT"]

const RAIL_HEAT := 26.0
const OVERDRIVE := 1.3
const OVERDRIVE_HEAT := 0.25
const HEAT_CRIT := 100.0
const HEAT_MAX := 130.0
const OVERLOAD_TIME := 12.0
const BREACH_TIME := 45.0
const AC_RANGE := 420.0
## Uplink restore percent per second per unit of repair power.
const UPLINK_RATE := 0.018
const RADAR_SCALE_M := 820.0


static func camera_bearing(cam: int) -> float:
	return cam * CAMERA_FOV


static func camera_for(angle: float) -> int:
	return int(fposmod(angle + CAMERA_FOV / 2.0, TAU) / CAMERA_FOV) % CAMERAS


static func crawl_time(s: int) -> float:
	return 2.0 + 0.9 * absf(SECTION_DECK[s] - SECTION_DECK[Section.COCKPIT])


static func bearing_deg(a: float) -> float:
	var d := fposmod(rad_to_deg(a), 360.0)
	return 0.0 if d >= 359.5 else d


## Angle difference wrapped to [-PI, PI].
static func angle_diff(a: float, b: float) -> float:
	return fposmod(a - b + PI, TAU) - PI
