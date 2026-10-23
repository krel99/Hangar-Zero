## Staged moments for screenshots (`HZ_SCENE=<name>`), ported from the Rust tool. Each
## stages enemies, damage and radio history on a fresh sim and lets it settle briefly.
class_name Shots
extends RefCounted

const SCENES := ["briefing", "combat", "boarded", "overload", "hangar"]


static func deg(d: float) -> float:
	return fposmod(deg_to_rad(d), TAU)


static func settle(scene: String) -> float:
	match scene:
		"briefing", "hangar": return 1.5
		"boarded": return 3.0
		_: return 2.4


static func _mk(sim: Sim, kind: int, serial: int, a: float, d: float, state: int, hp: float, max_hp: float, orbit: float) -> Entities.Enemy:
	var e := sim.spawn_enemy(kind, a, d)
	e.serial = serial
	e.hp = hp
	e.max_hp = max_hp
	e.hold_dist = d
	e.state = state
	e.timer = 1.5
	e.orbit = orbit
	e.drones = 2
	e.est_angle = a
	e.est_dist = d
	return e


static func _boarder(sim: Sim, section: int, age: float) -> void:
	var b := Entities.Boarder.new()
	b.section = section
	b.age = age
	sim.boarders.append(b)


static func _shell(sim: Sim, target: int, t: float, flight: float, angle: float) -> void:
	var s := Entities.Shell.new()
	s.target = target
	s.t = t
	s.flight = flight
	s.angle = angle
	sim.shells.append(s)


static func _drone(sim: Sim, angle: float, dist: float) -> void:
	var d := Entities.Drone.new()
	d.angle = angle
	d.dist = dist
	d.hp = 10.0
	sim.drones.append(d)


static func _say(sim: Sim, time: float, voice: int, text: String) -> void:
	sim.radio.log.append({"time": time, "voice": voice, "text": text})


static func stage_combat(scene: String, sim: Sim) -> void:
	var mech := sim.mech
	var mission := sim.mission
	mission.briefing = false
	mission.time = 131.0
	mission.next_wave = 5
	sim.radio.log.clear()
	var S := Rules.Section
	var V := Rules.Voice
	for l in [
		[0.0, V.SYSTEM, "COLD START. 4 FAULTS."], [2.0, V.CONTROL, "Uplink's dark. Get it up, hold the line."],
		[31.0, V.CONTROL, "Heavy contact bearing 302. Crab."], [47.0, V.PILOT, "Scout down."],
		[56.0, V.CONTROL, "Carrier bearing 118. Watch for ticks."], [66.0, V.ENEMY, "CRB-03 braced. Ranging."],
		[74.0, V.CHIEF, "Boarders. Point defence on."], [88.0, V.SYSTEM, "BOARDER ON COOLANT LINES."],
		[88.5, V.SYSTEM, "PARASITIC DRAW -6u"], [102.0, V.CONTROL, "Uplink hit. Progress lost."],
		[119.0, V.CHIEF, "Core's hot. Ease off."], [128.0, V.PILOT, "Rail's up. Crab first."],
	]:
		_say(sim, l[0], l[1], l[2])
	sim.radio.time = mission.time
	mission.serial = 7
	sim.enemies.clear()
	var K := Rules.Kind
	var E := Rules.EState
	var lock_serial := 5
	match scene:
		"overload":
			_mk(sim, K.SCOUT, 5, deg(28), 180, E.ENGAGED, 41, 60, 1)
			_mk(sim, K.SCOUT, 6, deg(64), 205, E.ENGAGED, 60, 60, -1)
			_mk(sim, K.ARTILLERY, 3, deg(36), 560, E.ENGAGED, 61, 200, 1)
			_mk(sim, K.CARRIER, 4, deg(118), 340, E.ENGAGED, 105, 150, 1)
			_boarder(sim, S.COOLANT, 10)
			_boarder(sim, S.REACTOR, 4)
			_shell(sim, S.REACTOR, 1.0, 6.5, deg(36))
			mech.heat = 112.0
			mech.overload = Rules.OVERLOAD_TIME - 3.5
			mech.hp[S.COOLANT] = 22.0
			mech.hp[S.REACTOR] = 48.0
			mech.hp[S.COCKPIT] = 61.0
			mech.hp[S.LEFT_LEG] = 0.0
			mech.hp[S.SENSORS] = 52.0
			mech.hp[S.UPLINK] = 71.0
			mech.repair_target = S.COOLANT
			mech.rail_charge = 0.55
			mech.heat_peak = 112.0
			_say(sim, 129.0, V.SYSTEM, "REACTOR OVERLOAD. 12s.")
			_say(sim, 129.2, V.CHIEF, "Vent it, scram it, or get out!")
		"boarded":
			_mk(sim, K.CARRIER, 4, deg(300), 345, E.ENGAGED, 96, 150, 1)
			_mk(sim, K.SCOUT, 6, deg(340), 470, E.APPROACH, 60, 60, -1)
			_boarder(sim, S.COOLANT, 10)
			_boarder(sim, S.COOLANT, 6)
			_boarder(sim, S.RAILGUN, 3)
			_drone(sim, deg(300), 220)
			mech.heat = 58.0
			mech.purge_cd = 9.0
			mech.fight_work = 6.0
			mech.view = deg(330)
			mech.hp[S.COOLANT] = 41.0
			mech.hp[S.RAILGUN] = 64.0
			mech.hp[S.SENSORS] = 80.0
			mech.hp[S.UPLINK] = 44.0
			mech.repair_target = S.COOLANT
			lock_serial = 4
			_say(sim, 127.0, V.SYSTEM, "BOARDER ON RAILGUN MOUNT.")
			_say(sim, 129.0, V.CHIEF, "Three ticks on you. Purge when it's cold enough.")
		_:
			# Mid-fight. The artillery sits in the dead LEFT quadrant: radar-only aim.
			_mk(sim, K.SCOUT, 5, deg(22), 230, E.ENGAGED, 41, 60, 1)
			_mk(sim, K.SCOUT, 6, deg(62), 330, E.APPROACH, 60, 60, -1)
			_mk(sim, K.ARTILLERY, 3, deg(292), 440, E.ENGAGED, 141, 200, 1)
			_mk(sim, K.CARRIER, 4, deg(118), 345, E.ENGAGED, 96, 150, 1)
			_drone(sim, deg(118), 150)
			_boarder(sim, S.COOLANT, 10)
			_shell(sim, S.LEFT_LEG, 0.6, 6.0, deg(292))
			mech.heat = 74.0
			mech.heat_peak = 88.0
			mech.hp[S.LEFT_LEG] = 31.0
			mech.hp[S.SENSORS] = 58.0
			mech.hp[S.COOLANT] = 52.0
			mech.hp[S.COCKPIT] = 77.0
			mech.hp[S.SHIELD_GEN] = 84.0
			mech.hp[S.RAILGUN] = 92.0
			mech.hp[S.UPLINK] = 47.0
			mech.rail_charge = 0.82
			mech.shield = 40.0
			mech.repair_target = S.UPLINK
			mech.kills = 2
			mech.view = deg(30)
			lock_serial = 3
	mech.target = -1
	for e in sim.enemies:
		if e.serial == lock_serial:
			mech.target = e.id


static func stage_hangar(campaign: Campaign, rng: Rng) -> void:
	campaign.mission = 3
	campaign.install(Campaign.Part.CAPACITOR_BANK)
	campaign.install(Campaign.Part.NAVY_RADIATOR)
	var S := Rules.Section
	campaign.max_hp[S.LEFT_LEG] = 72.0
	campaign.max_hp[S.SENSORS] = 81.0
	campaign.max_hp[S.COCKPIT] = 90.0
	campaign.max_hp[S.REACTOR] = 88.0
	campaign.last_report = {"number": 2, "time": 284.0, "kills": 9, "heat_peak": 104.0, "scars": [[S.LEFT_LEG, 14.0], [S.SENSORS, 9.0], [S.REACTOR, 6.0]]}
	campaign.roll_salvage(rng)
