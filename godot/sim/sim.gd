## The whole combat simulation for one operation. `step(dt)` runs the systems in a fixed
## order: clock, waves, power & heat, sensor fusion, weapons, enemy AI, drones,
## shells, boarders, outcome. The UI only reads state and raises `cmd_*` flags.
class_name Sim
extends RefCounted

const WAVES := [
	[4.0, [Rules.Kind.SCOUT, Rules.Kind.SCOUT]],
	[25.0, [Rules.Kind.ARTILLERY]],
	[50.0, [Rules.Kind.CARRIER, Rules.Kind.SCOUT]],
	[75.0, [Rules.Kind.SCOUT, Rules.Kind.SCOUT, Rules.Kind.SCOUT]],
	[105.0, [Rules.Kind.ARTILLERY, Rules.Kind.SCOUT, Rules.Kind.SCOUT]],
	[135.0, [Rules.Kind.CARRIER, Rules.Kind.ARTILLERY]],
	[165.0, [Rules.Kind.SCOUT, Rules.Kind.SCOUT, Rules.Kind.SCOUT]],
	[195.0, [Rules.Kind.ARTILLERY, Rules.Kind.CARRIER, Rules.Kind.SCOUT]],
]

const SCOUT_TARGETS := [Rules.Section.SENSORS, Rules.Section.COCKPIT, Rules.Section.LEFT_LEG, Rules.Section.RIGHT_LEG, Rules.Section.AUTOCANNON, Rules.Section.RAILGUN, Rules.Section.COOLANT]
## (section, weight)
const SHELL_TARGETS := [[Rules.Section.REACTOR, 3], [Rules.Section.LEFT_LEG, 2], [Rules.Section.RIGHT_LEG, 2], [Rules.Section.COCKPIT, 2], [Rules.Section.UPLINK, 2], [Rules.Section.SHIELD_GEN, 1], [Rules.Section.COOLANT, 2]]
const BOARD_SPOTS := [Rules.Section.SENSORS, Rules.Section.COOLANT, Rules.Section.RAILGUN, Rules.Section.AUTOCANNON, Rules.Section.SHIELD_GEN, Rules.Section.REACTOR, Rules.Section.LEFT_LEG, Rules.Section.RIGHT_LEG]

var campaign: Campaign
var rng: Rng
var mech: Mech
var mission: Mission
var radio := Radio.new()
var enemies: Array[Entities.Enemy] = []
var drones: Array[Entities.Drone] = []
var boarders: Array[Entities.Boarder] = []
var shells: Array[Entities.Shell] = []
var fx: Array[Entities.Fx] = []
var _next_id := 1

# Pilot commands raised by the UI, consumed by the step.
var cmd_fire_rail := false
var cmd_vent := false
var cmd_scram := false
var cmd_ecm := false
var cmd_eject := false
var cmd_purge := false


func _init(c: Campaign, seed_value: int) -> void:
	campaign = c
	rng = Rng.new(seed_value)
	mech = Mech.new(c)
	mission = Mission.new(c.mission)
	radio.say(Rules.Voice.SYSTEM, "COLD START. 4 FAULTS.")
	radio.say(Rules.Voice.CONTROL, "Uplink's dark. Get it up, hold the line.")
	if c.mission > 1:
		radio.say(Rules.Voice.CHIEF, "Op %02d. She's not what she was." % c.mission)
	else:
		radio.say(Rules.Voice.CHIEF, "Thirty years in a shed. Go easy on the core.")


func enemy_by_id(id: int) -> Entities.Enemy:
	for e in enemies:
		if e.id == id:
			return e
	return null


func target() -> Entities.Enemy:
	return enemy_by_id(mech.target) if mech.target >= 0 else null


func add_fx(kind: int, angle: float, dist: float, ttl: float) -> void:
	var f := Entities.Fx.new()
	f.kind = kind
	f.angle = angle
	f.dist = dist
	f.ttl = ttl
	fx.append(f)


## One frame. `dt` is clamped so a hitch does not teleport enemies.
func step(raw_dt: float) -> void:
	var dt := minf(raw_dt, 0.05)
	_tick_effects(dt)
	if not mission.running():
		return
	mission.time += dt
	radio.time = mission.time
	mech.sweep = fmod(mech.sweep + dt * 2.2, TAU)
	_spawn_waves()
	_power_and_heat(dt)
	_sensor_fusion()
	_weapons(dt)
	_enemy_ai(dt)
	_drones(dt)
	_shells(dt)
	_boarders(dt)
	_check_outcome()


func _tick_effects(dt: float) -> void:
	if mission.paused:
		return
	for f in fx:
		f.t += dt
	fx = fx.filter(func(f): return f.t < f.ttl)


# ---------------------------------------------------------------------------------------
# Waves
# ---------------------------------------------------------------------------------------

func spawn_enemy(kind: int, angle: float, dist: float) -> Entities.Enemy:
	var scale := 1.0 + 0.15 * maxi(mission.number - 1, 0)
	var hp: float = [60.0, 200.0, 150.0][kind]
	var hold: float
	match kind:
		Rules.Kind.SCOUT: hold = rng.range_f(150.0, 210.0)
		Rules.Kind.ARTILLERY: hold = rng.range_f(680.0, 760.0)
		_: hold = rng.range_f(320.0, 370.0)
	var e := Entities.Enemy.new()
	e.id = _next_id
	_next_id += 1
	e.kind = kind
	e.serial = mission.serial
	mission.serial += 1
	e.hp = hp * scale
	e.max_hp = hp * scale
	e.angle = angle
	e.dist = dist
	e.hold_dist = hold
	e.timer = rng.range_f(1.0, 3.0)
	e.orbit = 1.0 if rng.chance(0.5) else -1.0
	e.est_angle = angle
	e.est_dist = dist
	enemies.append(e)
	return e


func _spawn_waves() -> void:
	var t := mission.time
	var kinds: Array = []
	if mission.next_wave < WAVES.size():
		if WAVES[mission.next_wave][0] > t:
			return
		kinds = WAVES[mission.next_wave][1].duplicate()
	else:
		# Endless pressure after the scripted waves, while the uplink is still down.
		var last: float = WAVES.back()[0]
		var k := mission.next_wave - WAVES.size() + 1
		if not (t >= last + 30.0 * k and enemies.size() < 7):
			return
		match rng.index(3):
			0: kinds = [Rules.Kind.SCOUT, Rules.Kind.SCOUT]
			1: kinds = [Rules.Kind.ARTILLERY, Rules.Kind.SCOUT]
			_: kinds = [Rules.Kind.CARRIER]
	if mission.number >= 2 and mission.next_wave % 2 == 1:
		kinds.append(Rules.Kind.SCOUT)
	mission.next_wave += 1
	var bearing := rng.range_f(0.0, TAU)
	var heaviest := Rules.Kind.SCOUT
	for k in kinds:
		if [0, 2, 1][k] > [0, 2, 1][heaviest]:
			heaviest = k
	for k in kinds:
		var a := bearing + rng.range_f(-0.5, 0.5)
		var d := rng.range_f(950.0, 1100.0)
		spawn_enemy(k, a, d)
	var brg := "%03d" % int(Rules.bearing_deg(bearing))
	var line: String
	match heaviest:
		Rules.Kind.ARTILLERY: line = "Heavy contact bearing %s. Crab." % brg
		Rules.Kind.CARRIER: line = "Carrier bearing %s. Watch for ticks." % brg
		_: line = ("%d scouts bearing %s." % [kinds.size(), brg]) if kinds.size() > 1 else "Scout bearing %s." % brg
	radio.say(Rules.Voice.CONTROL, line)


# ---------------------------------------------------------------------------------------
# Reactor, heat, shields, repair, emergencies
# ---------------------------------------------------------------------------------------

## Everything the reactor bus and the damage state imply this instant.
static func derive(m: Mech, c: Campaign, n_boarders: int) -> Dictionary:
	var reactor_eff := 0.5 + 0.5 * m.frac(Rules.Section.REACTOR)
	var parasitic := n_boarders * 6.0 * c.hack_mult
	var available := maxf(c.reactor_output * reactor_eff - parasitic, 0.0)
	var draw := 0.0
	for a in m.alloc:
		draw += a
	var ceiling := available * Rules.OVERDRIVE
	var supply := ceiling / draw if draw > ceiling else 1.0
	if m.scram_t > 0.0:
		supply = 0.0
	var eff: Array[float] = []
	for i in Rules.SYSTEMS:
		eff.append(m.alloc[i] * supply)
	var delivered := draw * supply
	var overdrive := maxf(delivered - available, 0.0)

	var sens := eff[Rules.Sys.SENS]
	var sensor_factor := 0.25 if m.hp[Rules.Section.SENSORS] <= 0.0 else 0.35 + 0.65 * m.frac(Rules.Section.SENSORS)
	var radar_range := maxf((220.0 + sens * 26.0 + c.sensor_range) * sensor_factor, 120.0)
	var legs := (m.eff(Rules.Section.LEFT_LEG) + m.eff(Rules.Section.RIGHT_LEG)) * 0.5
	var leg_penalty := 0.4 if (m.hp[Rules.Section.LEFT_LEG] <= 0.0 or m.hp[Rules.Section.RIGHT_LEG] <= 0.0) else 1.0
	var evasion := clampf(eff[Rules.Sys.LOCO] / 30.0 * c.loco_mult * legs * leg_penalty, 0.0, 1.0)
	# Passive radiators plus whatever the cooling bus is wired for.
	var cooling := (2.0 + eff[Rules.Sys.COOL] * 0.6) * c.cooling_mult * m.eff(Rules.Section.COOLANT)
	var shield_cap := eff[Rules.Sys.SHLD] * 4.0 * c.shield_mult * m.eff(Rules.Section.SHIELD_GEN)
	var rail_rate := eff[Rules.Sys.RAIL] / 40.0 * 0.4 * c.rail_charge_mult * m.eff(Rules.Section.RAILGUN)
	var ac_dps := eff[Rules.Sys.AC] * 0.7 * c.ac_mult * m.eff(Rules.Section.AUTOCANNON)
	var repair_rate := eff[Rules.Sys.REPR] * 0.16 * c.repair_mult

	var heat_in := (1.0 + delivered * 0.025) * c.base_heat_mult + overdrive * Rules.OVERDRIVE_HEAT
	heat_in += (1.0 - m.frac(Rules.Section.REACTOR)) * 4.0
	if m.frac(Rules.Section.COOLANT) < 0.35:
		heat_in += 2.5
	if m.interlocks_failed:
		heat_in += 3.0
	if m.ac_firing:
		heat_in += eff[Rules.Sys.AC] * 0.2
	var heat_out := cooling
	if m.scram_t > 0.0:
		heat_out += 10.0
		heat_in = 0.5
	if m.vent_t > 0.0:
		heat_out += 20.0

	return {
		"available": available, "parasitic": parasitic, "draw": draw, "supply": supply, "eff": eff,
		"radar_range": radar_range, "id_range": radar_range * 0.65, "evasion": evasion, "cooling": cooling,
		"heat_in": heat_in, "heat_out": heat_out, "shield_cap": shield_cap, "rail_rate": rail_rate,
		"ac_dps": ac_dps, "repair_rate": repair_rate,
	}


func _power_and_heat(dt: float) -> void:
	var st := derive(mech, campaign, boarders.size())
	var eff: Array = st.eff
	mech.heat = clampf(mech.heat + (st.heat_in - st.heat_out) * dt, 0.0, Rules.HEAT_MAX)
	mech.heat_peak = maxf(mech.heat_peak, mech.heat)
	if mech.heat > 90.0:
		# Lines cook when the core runs hot.
		mech.damage(Rules.Section.COOLANT, 0.6 * dt)

	mech.shield_hit_t += dt
	if mech.shields_down_t > 0.0:
		mech.shields_down_t -= dt
		mech.shield = 0.0
	else:
		var regen: float = eff[Rules.Sys.SHLD] * (0.5 if mech.shield_hit_t > 1.5 else 0.15)
		mech.shield = minf(mech.shield + regen * dt, st.shield_cap)

	# Repair drones: pry boarders off the target section first, then fix it. At the
	# sensor head they also rewire the worst camera.
	var rt := mech.repair_target
	if mech.alloc[Rules.Sys.REPR] > 0.0:
		var here: Array = boarders.filter(func(b): return b.section == rt)
		if not here.is_empty():
			mech.fight_work += st.repair_rate * dt
			if mech.fight_work >= Rules.BOARDER_WORK:
				boarders.erase(here[0])
				mech.fight_work = 0.0
				radio.say(Rules.Voice.SYSTEM, "DRONES CLEARED %s." % Rules.SECTION_SHORT[rt])
		else:
			mech.fight_work = 0.0
			if rt == Rules.Section.UPLINK:
				mech.hp[rt] = minf(mech.hp[rt] + eff[Rules.Sys.REPR] * Rules.UPLINK_RATE * campaign.repair_mult * dt, 100.0)
			else:
				mech.hp[rt] = minf(mech.hp[rt] + st.repair_rate * dt, mech.max_hp[rt])
			if rt == Rules.Section.SENSORS:
				var cam := -1
				for c in Rules.CAMERAS:
					if mech.cameras[c] < Rules.CAMERA_HP and (cam < 0 or mech.cameras[c] < mech.cameras[cam]):
						cam = c
				if cam >= 0:
					var was_dead := mech.cameras[cam] < Rules.CAMERA_MIN
					mech.cameras[cam] = minf(mech.cameras[cam] + st.repair_rate * 1.2 * dt, Rules.CAMERA_HP)
					if was_dead and mech.cameras[cam] >= Rules.CAMERA_MIN:
						radio.say(Rules.Voice.SYSTEM, "%s CAMERA ONLINE." % Rules.CAMERA_NAMES[cam])

	mech.vent_t = maxf(mech.vent_t - dt, 0.0)
	mech.vent_cd = maxf(mech.vent_cd - dt, 0.0)
	mech.scram_t = maxf(mech.scram_t - dt, 0.0)
	mech.scram_cd = maxf(mech.scram_cd - dt, 0.0)
	mech.ecm_cd = maxf(mech.ecm_cd - dt, 0.0)
	mech.purge_cd = maxf(mech.purge_cd - dt, 0.0)
	mech.purge_t = maxf(mech.purge_t - dt, 0.0)
	mech.hit_flash = maxf(mech.hit_flash - dt * 2.5, 0.0)
	for i in Rules.SECTIONS:
		mech.hit_t[i] = maxf(mech.hit_t[i] - dt * 1.5, 0.0)
	mech.shake = maxf(mech.shake - dt * 3.0, 0.0)

	if cmd_vent:
		cmd_vent = false
		if mech.vent_cd <= 0.0:
			mech.vent_t = 2.0
			mech.vent_cd = 25.0
			mech.shields_down_t = 6.0
			mech.damage(Rules.Section.COOLANT, 4.0)
			radio.say(Rules.Voice.SYSTEM, "VENTING. SHIELDS DOWN 6s.")
	if cmd_scram:
		cmd_scram = false
		if mech.scram_cd <= 0.0:
			mech.scram_t = 6.0
			mech.scram_cd = 30.0
			radio.say(Rules.Voice.SYSTEM, "SCRAM. ALL BUSES DEAD 6s.")

	# Cockpit breach
	if mech.frac(Rules.Section.COCKPIT) < 0.25 and not mech.interlocks_failed:
		if mech.breach < 0.0:
			mech.breach = Rules.BREACH_TIME
			radio.say(Rules.Voice.SYSTEM, "COCKPIT BREACH. 45s TO INTERLOCK FAILURE.")
			radio.say(Rules.Voice.CHIEF, "Patch the canopy, kid.")
	elif mech.frac(Rules.Section.COCKPIT) >= 0.35 and mech.breach >= 0.0:
		mech.breach = -1.0
		radio.say(Rules.Voice.SYSTEM, "COCKPIT SEALED.")
	if mech.breach >= 0.0:
		mech.breach -= dt
		if mech.breach <= 0.0:
			mech.breach = -1.0
			mech.interlocks_failed = true
			radio.say(Rules.Voice.SYSTEM, "INTERLOCKS FAILED. CORE RUNNING AWAY.")

	# Reactor overload: the failure state
	if mech.heat >= Rules.HEAT_CRIT:
		if mech.overload < 0.0:
			mech.overload = Rules.OVERLOAD_TIME
			radio.say(Rules.Voice.SYSTEM, "REACTOR OVERLOAD. 12s.")
			radio.say(Rules.Voice.CHIEF, "Vent it, scram it, or get out!")
	elif mech.heat < 92.0 and mech.overload >= 0.0:
		mech.overload = -1.0
		radio.say(Rules.Voice.SYSTEM, "Core cooling. Overload aborted.")
		radio.say(Rules.Voice.PILOT, "...Breathe.")
	if mech.overload >= 0.0:
		mech.overload -= dt
	if mech.heat > 80.0 and mission.warn_once("heat80"):
		radio.say(Rules.Voice.CHIEF, "Core's hot. Ease off.")
	if boarders.size() > 0 and mission.warn_once("hack"):
		radio.say(Rules.Voice.SYSTEM, "PARASITIC DRAW -%du" % int(st.parasitic))
	mech.st = st


# ---------------------------------------------------------------------------------------
# Radar: camera-covered contacts are exact; the rest are blips re-sampled per sweep
# ---------------------------------------------------------------------------------------

func _sensor_fusion() -> void:
	var sens: float = mech.st.eff[Rules.Sys.SENS]
	var head := mech.frac(Rules.Section.SENSORS)
	var base_err := (3.0 + 14.0 * (1.0 - minf(sens / 20.0, 1.0))) * (1.6 - 0.6 * head) + campaign.radar_err
	for e in enemies:
		var covered := mech.covers(e.angle) and e.dist <= 950.0
		if covered:
			e.covered = true
			e.err = 0.0
			e.est_angle = e.angle
			e.est_dist = e.dist
			continue
		var was_covered := e.covered
		e.covered = false
		var since := fposmod(mech.sweep - e.angle, TAU)
		var passed := (since < 0.12 and mission.time - e.last_sweep > 1.0) or was_covered
		if passed or e.last_sweep < -5.0:
			e.last_sweep = mission.time
			var err := deg_to_rad(base_err)
			e.err = err
			e.est_angle = fposmod(e.angle + rng.range_f(-err, err), TAU)
			e.est_dist = maxf(e.dist * (1.0 + rng.range_f(-0.12, 0.12) * (base_err / 17.0)), 20.0)


# ---------------------------------------------------------------------------------------
# Pilot out of the chair
# ---------------------------------------------------------------------------------------

# ---------------------------------------------------------------------------------------
# Weapons
# ---------------------------------------------------------------------------------------

func _weapons(dt: float) -> void:
	var st := mech.st
	if mech.target >= 0 and enemy_by_id(mech.target) == null:
		mech.target = -1

	# Railgun
	mech.rail_charge = minf(mech.rail_charge + st.rail_rate * dt, 1.0)
	var fire := cmd_fire_rail
	cmd_fire_rail = false
	if fire and mech.rail_charge >= 1.0:
		var e := target()
		if e and e.dist <= st.radar_range:
			mech.rail_charge = 0.0
			mech.heat += Rules.RAIL_HEAT + campaign.rail_heat
			mech.shake = 1.0
			var p := 0.92 * (0.5 + 0.5 * mech.eff(Rules.Section.SENSORS))
			if e.kind == Rules.Kind.SCOUT:
				p -= 0.15
			p *= e.aim_factor()
			# The shot goes where the radar says the target is.
			add_fx(Rules.Fx.RAIL, e.est_angle, e.est_dist, 0.45)
			if rng.chance(p):
				e.hp -= 75.0 * campaign.rail_damage_mult
				e.hit_flash = 0.4
				add_fx(Rules.Fx.EXPLOSION, e.angle, e.dist, 0.7)
			else:
				radio.say(Rules.Voice.PILOT, "Miss.")

	# Autocannon
	mech.ac_firing = false
	if st.ac_dps > 0.0 and mech.weapons_free:
		mech.ac_burst_t += dt
		var drone_target: Entities.Drone = null
		if mech.point_defence:
			var best := Rules.AC_RANGE
			for d in drones:
				if d.dist < best:
					best = d.dist
					drone_target = d
		if drone_target:
			drone_target.hp -= st.ac_dps * 1.4 * dt
			mech.ac_firing = true
			if mech.ac_burst_t > 0.12:
				add_fx(Rules.Fx.TRACER, drone_target.angle, drone_target.dist, 0.15)
				mech.ac_burst_t = 0.0
			if drone_target.hp <= 0.0:
				add_fx(Rules.Fx.EXPLOSION, drone_target.angle, drone_target.dist, 0.35)
				drones.erase(drone_target)
		else:
			var chosen := target()
			if chosen == null or chosen.dist > Rules.AC_RANGE:
				chosen = null
				var lim := minf(Rules.AC_RANGE, st.radar_range)
				for e in enemies:
					if e.dist <= lim and (chosen == null or e.dist < chosen.dist):
						chosen = e
			if chosen:
				var falloff := 1.0 - (chosen.dist / Rules.AC_RANGE) * 0.5
				var blind := 1.0 if chosen.covered else 0.5
				chosen.hp -= st.ac_dps * falloff * blind * dt
				chosen.hit_flash = maxf(chosen.hit_flash, 0.08)
				mech.ac_firing = true
				if mech.ac_burst_t > 0.12:
					add_fx(Rules.Fx.TRACER, chosen.est_angle + rng.range_f(-0.02, 0.02), chosen.est_dist, 0.15)
					mech.ac_burst_t = 0.0

	# Electronic warfare
	if cmd_ecm:
		cmd_ecm = false
		if mech.ecm_cd <= 0.0 and st.eff[Rules.Sys.SENS] >= 5.0:
			mech.ecm_cd = 28.0
			mech.heat += 12.0
			add_fx(Rules.Fx.ECM, 0.0, 0.0, 1.0)
			var fried := 0
			for d in drones.duplicate():
				if d.dist < 300.0:
					add_fx(Rules.Fx.EXPLOSION, d.angle, d.dist, 0.35)
					drones.erase(d)
					fried += 1
			for b in boarders.duplicate():
				if rng.chance(0.6):
					boarders.erase(b)
					fried += 1
			for e in enemies:
				if e.dist < 400.0:
					e.stun = 3.5
			radio.say(Rules.Voice.SYSTEM, "ECM. %d drones fried." % fried)

	# Purge: run the reactor through the hull plating. Every tick on it dies; the core pays.
	if cmd_purge:
		cmd_purge = false
		if mech.purge_cd <= 0.0 and mech.scram_t <= 0.0:
			mech.purge_cd = Rules.PURGE_CD
			mech.purge_t = 0.6
			mech.heat += Rules.PURGE_HEAT
			var n := boarders.size()
			boarders.clear()
			mech.fight_work = 0.0
			radio.say(Rules.Voice.SYSTEM, "PURGE. %d boarders fried." % n if n > 0 else "PURGE. Hull clear.")

	# Kills
	for e in enemies.duplicate():
		if e.hp <= 0.0:
			add_fx(Rules.Fx.EXPLOSION, e.angle, e.dist, 1.1)
			enemies.erase(e)
			mech.kills += 1
			if mech.target == e.id:
				# Acquire the nearest remaining contact so the next shot is not lost to a tap.
				var nearest: Entities.Enemy = null
				for o in enemies:
					if o.hp > 0.0 and o.dist <= st.radar_range and (nearest == null or o.dist < nearest.dist):
						nearest = o
				mech.target = nearest.id if nearest else -1
			radio.say(Rules.Voice.PILOT, ["Scout down.", "Crab down.", "Carrier down."][e.kind])


# ---------------------------------------------------------------------------------------
# Enemies
# ---------------------------------------------------------------------------------------

## Returns the camera that died, if the hit killed one (else -1).
func hit_mech(s: int, dmg: float, from: float, has_from: bool = true) -> int:
	var absorbed := minf(dmg, mech.shield)
	mech.shield -= absorbed
	mech.heat += absorbed * 0.35
	mech.shield_hit_t = 0.0
	var rest := dmg - absorbed
	var died := -1
	if rest > 0.0:
		mech.damage(s, rest)
		mission.damage_taken[s] += rest
		mech.hit_flash = 1.0
		# Fire that reaches the sensor head also chews the camera facing the shooter.
		if s == Rules.Section.SENSORS and has_from:
			var cam := Rules.camera_for(from)
			var was_ok := mech.cameras[cam] >= Rules.CAMERA_MIN
			mech.cameras[cam] = maxf(mech.cameras[cam] - rest * 1.5, 0.0)
			if was_ok and mech.cameras[cam] < Rules.CAMERA_MIN:
				died = cam
	return died


func _enemy_ai(dt: float) -> void:
	var evasion: float = mech.st.evasion
	for e in enemies:
		e.hit_flash = maxf(e.hit_flash - dt, 0.0)
		e.reveal = maxf(e.reveal - dt, 0.0)
		if e.stun > 0.0:
			e.stun -= dt
			continue
		e.timer -= dt
		match e.kind:
			Rules.Kind.SCOUT:
				if e.dist > e.hold_dist:
					e.dist -= 40.0 * dt
					e.angle += 0.12 * e.orbit * dt
				else:
					e.angle += 0.22 * e.orbit * dt
					e.dist += rng.range_f(-1.0, 1.0) * 20.0 * dt
				# Jackals open fire while still closing.
				if e.dist < 400.0 and e.timer <= 0.0:
					e.timer = rng.range_f(1.6, 2.4)
					add_fx(Rules.Fx.MUZZLE, e.angle, e.dist, 0.2)
					var close := 1.0 if e.dist < 250.0 else 0.7
					if rng.chance(0.5 * close * (1.0 - 0.6 * evasion)):
						var s: int = SCOUT_TARGETS[rng.index(SCOUT_TARGETS.size())]
						var cam := hit_mech(s, rng.range_f(4.0, 7.0), e.angle)
						if cam >= 0:
							radio.say(Rules.Voice.SYSTEM, "CAMERA %s LOST." % Rules.CAMERA_NAMES[cam])
			Rules.Kind.ARTILLERY:
				match e.state:
					Rules.EState.APPROACH:
						e.dist -= 14.0 * dt
						if e.dist <= e.hold_dist:
							e.state = Rules.EState.DEPLOYING
							e.timer = 3.0
					Rules.EState.DEPLOYING:
						if e.timer <= 0.0:
							e.state = Rules.EState.ENGAGED
							e.timer = 0.5
							radio.say(Rules.Voice.ENEMY, "%s braced. Ranging." % e.callsign())
					Rules.EState.ENGAGED:
						if e.timer <= 0.0:
							e.timer = rng.range_f(8.0, 10.0)
							if mech.covers(e.angle):
								e.reveal = 3.0
							add_fx(Rules.Fx.MUZZLE, e.angle, e.dist, 0.4)
							var total := 0
							for st_w in SHELL_TARGETS:
								total += st_w[1]
							var roll := rng.index(total)
							var tgt: int = Rules.Section.REACTOR
							for st_w in SHELL_TARGETS:
								if roll < st_w[1]:
									tgt = st_w[0]
									break
								roll -= st_w[1]
							var sh := Entities.Shell.new()
							sh.target = tgt
							sh.angle = e.angle
							shells.append(sh)
			Rules.Kind.CARRIER:
				if e.dist > e.hold_dist:
					e.dist -= 24.0 * dt
				else:
					e.angle += 0.03 * e.orbit * dt
					if e.timer <= 0.0 and e.drones > 0:
						e.timer = rng.range_f(4.5, 6.0)
						e.drones -= 1
						var d := Entities.Drone.new()
						d.angle = e.angle
						d.dist = e.dist - 10.0
						drones.append(d)
						if mission.warn_once("first_drone"):
							radio.say(Rules.Voice.CHIEF, "Boarders. Point defence on.")
		e.angle = fposmod(e.angle, TAU)


func _drones(dt: float) -> void:
	for d in drones.duplicate():
		d.dist -= 70.0 * dt
		d.angle += sin(d.dist * 0.05) * 0.2 * dt
		if d.dist <= 8.0:
			drones.erase(d)
			var b := Entities.Boarder.new()
			b.section = BOARD_SPOTS[rng.index(BOARD_SPOTS.size())]
			boarders.append(b)
			radio.say(Rules.Voice.SYSTEM, "BOARDER ON %s." % Rules.SECTION_NAME[b.section])


func _shells(dt: float) -> void:
	for s in shells.duplicate():
		s.t += dt
		if s.t < s.flight:
			continue
		shells.erase(s)
		var p: float = 0.7 - 0.5 * mech.st.evasion
		if rng.chance(p):
			var dmg := 8.0 if s.target == Rules.Section.UPLINK else rng.range_f(14.0, 20.0)
			var d1 := hit_mech(s.target, dmg, s.angle)
			var splash: int = rng.index(9)
			var d2 := hit_mech(splash, 5.0, s.angle)
			var cam := d1 if d1 >= 0 else d2
			if cam >= 0:
				radio.say(Rules.Voice.SYSTEM, "CAMERA %s LOST." % Rules.CAMERA_NAMES[cam])
			mech.shake = 1.4
			add_fx(Rules.Fx.SHELL_IMPACT, s.angle, 30.0, 0.8)
			if s.target == Rules.Section.UPLINK:
				radio.say(Rules.Voice.CONTROL, "Uplink hit. Progress lost.")
		else:
			add_fx(Rules.Fx.SHELL_IMPACT, s.angle + 0.4, 120.0, 0.8)
			if mission.warn_once("dodge"):
				radio.say(Rules.Voice.PILOT, "Stepped clear.")


func _boarders(dt: float) -> void:
	for b in boarders:
		b.age += dt
		var dmg := 1.2 * campaign.hack_mult * dt
		mech.damage(b.section, dmg)
		mission.damage_taken[b.section] += dmg


func _check_outcome() -> void:
	if mech.uplink() >= 100.0:
		mission.outcome = Rules.Outcome.VICTORY
		radio.say(Rules.Voice.CONTROL, "Uplink green. Bring her home.")
	elif mech.overload >= 0.0 and mech.overload <= 0.0:
		mission.outcome = Rules.Outcome.MELTDOWN
		mech.heat = Rules.HEAT_MAX
		radio.say(Rules.Voice.SYSTEM, "CONTAINMENT LOST.")
	elif cmd_eject:
		cmd_eject = false
		mission.outcome = Rules.Outcome.EJECTED
		radio.say(Rules.Voice.PILOT, "Punching out!")
