## Persistent state between operations: scars, salvaged parts, the energy matrix and how
## much of the original walking fortress is left.
class_name Campaign
extends RefCounted

enum Maker { BASTION, KESSLER_VOLT, OSTRAVA, THALASSA, CIVIC_TRANSIT, JACKAL, HIVE, CRAB }
const MAKER_NAME := ["BASTION WORKS", "KESSLER-VOLT", "OSTRAVA HEAVY", "THALASSA NAVAL", "CIVIC TRANSIT", "JACKAL (ENEMY)", "HIVE (GRAFT)", "CRAB (ENEMY)"]
const MAKER_ACCENT := [
	Color8(232, 168, 56), Color8(96, 200, 255), Color8(214, 96, 64), Color8(120, 220, 190),
	Color8(240, 220, 120), Color8(190, 210, 90), Color8(190, 110, 240), Color8(200, 140, 110),
]

enum Part { CAPACITOR_BANK, OSTRAVA_ACTUATOR, JACKAL_DISH, NAVY_RADIATOR, HIVE_REACTOR, CRAB_DAMPERS, TRAM_BATTERY, ORIGINAL_SPARES, FUEL_RODS, CCTV_HEAD }

## name, maker, section, pros, cons, flavor, icon
const PARTS := {
	Part.CAPACITOR_BANK: ["KV-9 Capacitor Bank", Maker.KESSLER_VOLT, Rules.Section.RAILGUN, "+35% rail charge", "+6 heat per shot", "Pulled from a substation. Still hums.", 0],
	Part.OSTRAVA_ACTUATOR: ["Ostrava Leg Actuator", Maker.OSTRAVA, Rules.Section.LEFT_LEG, "Weak leg to 110 max, +25% loco", "Mismatched gait", "Mining-walker knee. Twice the torque, half the grace.", 1],
	Part.JACKAL_DISH: ["Jackal Sensor Dish", Maker.JACKAL, Rules.Section.SENSORS, "+140m radar, square scope", "Enemy firmware: sensors -15 max", "Peeled off a dead scout. It still pings its old pack.", 2],
	Part.NAVY_RADIATOR: ["Navy Surplus Radiator", Maker.THALASSA, Rules.Section.COOLANT, "+30% cooling, coolant max 120", "Salt-crusted: loco -10%", "Built to cool a destroyer's turbines.", 3],
	Part.HIVE_REACTOR: ["Hive Reactor Sleeve", Maker.HIVE, Rules.Section.REACTOR, "+15 reactor, +40% repair", "Boarders drain double", "It grew into the reactor housing overnight.", 4],
	Part.CRAB_DAMPERS: ["Crab Recoil Dampers", Maker.CRAB, Rules.Section.AUTOCANNON, "+30% autocannon damage", "A/C mount max -10", "Artillery shocks, cut down with a plasma torch.", 5],
	Part.TRAM_BATTERY: ["Civic Tram Battery", Maker.CIVIC_TRANSIT, Rules.Section.SHIELD_GEN, "+30% shield capacity", "+10% base core heat", "Line 4 to the harbour. Nobody rides it anymore.", 6],
	Part.ORIGINAL_SPARES: ["Original Spares", Maker.BASTION, Rules.Section.COCKPIT, "Heals scars on 2 worst sections", "No upgrade. As she was.", "A sealed crate stamped 1987.", 7],
	Part.FUEL_RODS: ["Overclocked Fuel Rods", Maker.KESSLER_VOLT, Rules.Section.REACTOR, "+20 reactor output", "+25% base core heat", "Enrichment unknown. Geiger says 'yes'.", 8],
	Part.CCTV_HEAD: ["Transit CCTV Cluster", Maker.CIVIC_TRANSIT, Rules.Section.SENSORS, "All 4 cameras restored", "Radar-only error +2 deg", "Watched platform 4 for twenty years. Now it watches for you.", 9],
}

## Modifiers from installed salvage.
var reactor_output := 100.0
var rail_charge_mult := 1.0
var rail_heat := 0.0
var rail_damage_mult := 1.0
var sensor_range := 0.0
var cooling_mult := 1.0
var shield_mult := 1.0
var repair_mult := 1.0
var loco_mult := 1.0
var ac_mult := 1.0
var hack_mult := 1.0
var base_heat_mult := 1.0
## Extra bearing error, in degrees, on radar-only contacts.
var radar_err := 0.0

var mission := 1
var max_hp: Array[float] = []
var makers: Array[int] = []
var installed: Array[int] = []
var originality := 100.0
var salvage: Array[int] = []
var last_report: Dictionary = {}
## Hull camera health carried between operations.
var cameras: Array[float] = []
## Energy matrix: which systems are wired to the reactor. Set in the hangar.
var matrix: Array[bool] = []


func _init() -> void:
	# She was never fully repaired after the war: the left camera is dead and three
	# sections carry old scars before the first operation.
	max_hp.resize(Rules.SECTIONS)
	max_hp.fill(100.0)
	max_hp[Rules.Section.LEFT_LEG] = 70.0
	max_hp[Rules.Section.SENSORS] = 80.0
	max_hp[Rules.Section.COOLANT] = 85.0
	makers.resize(Rules.SECTIONS)
	makers.fill(Maker.BASTION)
	cameras.assign([Rules.CAMERA_HP, Rules.CAMERA_HP, Rules.CAMERA_HP, 0.0])
	matrix.assign(Rules.DEFAULT_MATRIX)


## Power each bus gets in battle: its fixed cost if wired, else nothing.
func alloc() -> Array[float]:
	var a: Array[float] = []
	for i in Rules.SYSTEMS:
		a.append(Rules.MATRIX_COST[i] if matrix[i] else 0.0)
	return a


func matrix_draw() -> float:
	var d := 0.0
	for i in Rules.SYSTEMS:
		if matrix[i]:
			d += Rules.MATRIX_COST[i]
	return d


static func part_name(id: int) -> String:
	return PARTS[id][0]


static func part_maker(id: int) -> int:
	return PARTS[id][1]


static func part_section(id: int) -> int:
	return PARTS[id][2]


## Permanent wear: every mission leaves scars proportional to damage taken.
func apply_scars(mission_number: int, mission_time: float, damage_taken: Array, kills: int, heat_peak: float) -> void:
	var scars: Array = []
	for s in Rules.SECTIONS:
		if s == Rules.Section.UPLINK:
			continue
		var scar := floorf(minf(damage_taken[s] * 0.18, 25.0))
		if scar >= 1.0:
			max_hp[s] = maxf(max_hp[s] - scar, 40.0)
			scars.append([s, scar])
	last_report = {"number": mission_number, "time": mission_time, "kills": kills, "heat_peak": heat_peak, "scars": scars}


func roll_salvage(rng: Rng) -> void:
	var pool: Array[int] = []
	for id in PARTS.keys():
		if id == Part.CCTV_HEAD:
			continue
		if not installed.has(id) or id == Part.ORIGINAL_SPARES:
			pool.append(id)
	salvage.clear()
	for c in cameras:
		if c < Rules.CAMERA_HP:
			salvage.append(Part.CCTV_HEAD)
			break
	while salvage.size() < 5 and not pool.is_empty():
		salvage.append(pool.pop_at(rng.index(pool.size())))


func install(id: int) -> void:
	var section: int = part_section(id)
	var maker: int = part_maker(id)
	match id:
		Part.CAPACITOR_BANK:
			rail_charge_mult *= 1.35
			rail_heat += 6.0
		Part.OSTRAVA_ACTUATOR:
			var l := Rules.Section.LEFT_LEG
			var r := Rules.Section.RIGHT_LEG
			var i := l if max_hp[l] <= max_hp[r] else r
			max_hp[i] = 110.0
			makers[i] = Maker.OSTRAVA
			loco_mult *= 1.25
		Part.JACKAL_DISH:
			sensor_range += 140.0
			max_hp[Rules.Section.SENSORS] = maxf(max_hp[Rules.Section.SENSORS] - 15.0, 40.0)
		Part.NAVY_RADIATOR:
			cooling_mult *= 1.3
			max_hp[Rules.Section.COOLANT] = 120.0
			loco_mult *= 0.9
		Part.HIVE_REACTOR:
			reactor_output += 15.0
			repair_mult *= 1.4
			hack_mult *= 2.0
		Part.CRAB_DAMPERS:
			ac_mult *= 1.3
			max_hp[Rules.Section.AUTOCANNON] = maxf(max_hp[Rules.Section.AUTOCANNON] - 10.0, 40.0)
		Part.TRAM_BATTERY:
			shield_mult *= 1.3
			base_heat_mult *= 1.1
		Part.ORIGINAL_SPARES:
			var worn: Array[int] = []
			for s in Rules.SECTIONS:
				if s != Rules.Section.UPLINK:
					worn.append(s)
			worn.sort_custom(func(a, b): return max_hp[a] < max_hp[b])
			for s in worn.slice(0, 2):
				max_hp[s] = maxf(max_hp[s], 100.0)
				makers[s] = Maker.BASTION
			originality = minf(originality + 6.0, 100.0)
		Part.FUEL_RODS:
			reactor_output += 20.0
			base_heat_mult *= 1.25
		Part.CCTV_HEAD:
			cameras.fill(Rules.CAMERA_HP)
			radar_err += 2.0
	if id != Part.OSTRAVA_ACTUATOR and id != Part.ORIGINAL_SPARES:
		makers[section] = maker
	if maker != Maker.BASTION:
		originality = maxf(originality - 11.0, 0.0)
	installed.append(id)


## The machine's identity, as the hangar describes it.
func identity() -> String:
	var o := int(originality)
	if o >= 90:
		return "WF-7 BASTION, as she left the factory"
	if o >= 70:
		return "BASTION, patched with salvage"
	if o >= 45:
		return "Half Bastion, half scrapyard"
	if o >= 20:
		return "Something wearing Bastion's skin"
	return "Unrecognisable. The city calls it 'the Chimera'"
