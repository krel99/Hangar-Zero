## Headless simulation tests. Run from the repo root:
##   godot --headless --path godot -s tests/test_sim.gd
extends SceneTree

var failures := 0


func check(cond: bool, msg: String) -> void:
	if cond:
		print("  ok   ", msg)
	else:
		failures += 1
		push_error("  FAIL ", msg)
		print("  FAIL ", msg)


func run_mission(seed_value: int, with_pilot: bool) -> Dictionary:
	var c := Campaign.new()
	var sim := Sim.new(c, seed_value)
	sim.mission.briefing = false
	var steps := 0
	while steps < 20 * 600 and sim.mission.outcome == Rules.Outcome.NONE:
		if with_pilot:
			Autopilot.act(sim)
		sim.step(0.05)
		steps += 1
	var worst := 1.0
	for s in 9:
		worst = minf(worst, sim.mech.frac(s))
	var dmg := 0.0
	for d in sim.mission.damage_taken:
		dmg += d
	return {"outcome": sim.mission.outcome, "time": sim.mission.time, "uplink": sim.mech.uplink(), "peak": sim.mech.heat_peak, "kills": sim.mech.kills, "damage": dmg, "worst": worst}


func _init() -> void:
	print("== idle pilot still produces a result")
	var r := run_mission(3, false)
	check(r.outcome != Rules.Outcome.NONE or r.uplink > 0.0, "something happened in %.0fs" % r.time)

	print("== autopilot missions land in the target length")
	var wins := 0
	for seed_value in [1, 2, 3, 4]:
		r = run_mission(seed_value, true)
		print("  seed %d: %s after %.0fs, uplink %.0f%%, peak heat %.0f, kills %d, damage %.0f, worst %.2f" % [
			seed_value, Rules.Outcome.keys()[r.outcome], r.time, r.uplink, r.peak, r.kills, r.damage, r.worst])
		check(r.outcome != Rules.Outcome.NONE, "seed %d resolved" % seed_value)
		check(r.time >= 150.0 and r.time <= 480.0, "seed %d took %.0fs (want 150-480)" % [seed_value, r.time])
		if r.outcome == Rules.Outcome.VICTORY:
			wins += 1
	check(wins >= 2, "autopilot wins at least 2 of 4 (%d)" % wins)

	print("== overdrive costs heat")
	var c := Campaign.new()
	var m := Mech.new(c)
	var calm := Sim.derive(m, c, 0)
	for i in Rules.SYSTEMS:
		m.alloc[i] = [30.0, 20.0, 40.0, 25.0, 35.0, 25.0, 30.0][i]
	var hot := Sim.derive(m, c, 0)
	check(hot.supply < 1.0, "205 units cannot all be delivered")
	check(hot.heat_in > calm.heat_in + 5.0, "heat climbs under overdrive")

	print("== cameras")
	check(Rules.camera_for(0.0) == 0 and Rules.camera_for(PI / 2.0) == 1 and Rules.camera_for(PI) == 2 and Rules.camera_for(-PI / 2.0) == 3, "quadrants map to cameras")
	check(not m.camera_ok(3) and m.cameras_up() == 3, "op 1 starts with the LEFT camera dead")

	print("== boarders: purge and drones")
	var bs := Sim.new(Campaign.new(), 11)
	bs.mission.briefing = false
	for sec in [Rules.Section.COOLANT, Rules.Section.COOLANT, Rules.Section.RAILGUN]:
		var b := Entities.Boarder.new()
		b.section = sec
		bs.boarders.append(b)
	bs.mech.repair_target = Rules.Section.COOLANT
	var hp0: float = bs.mech.hp[Rules.Section.COOLANT]
	for i in 200:
		bs.step(0.05)
	check(bs.boarders.size() <= 2 and bs.boarders.size() > 0, "drones on the boarded section pry ticks off, one at a time (%d left after 10 s)" % bs.boarders.size())
	check(bs.mech.hp[Rules.Section.COOLANT] < hp0, "the section still took damage meanwhile")
	var heat0 := bs.mech.heat
	bs.cmd_purge = true
	bs.step(0.05)
	check(bs.boarders.is_empty(), "PURGE fries every boarder")
	check(bs.mech.heat > heat0 + Rules.PURGE_HEAT * 0.9, "PURGE costs heat")
	check(bs.mech.purge_cd > 0.0, "PURGE has a cooldown")
	bs.cmd_purge = true
	bs.step(0.05)
	check(absf(bs.mech.heat - heat0 - Rules.PURGE_HEAT) < 2.0, "a second PURGE on cooldown does nothing")
	bs.mech.repair_target = Rules.Section.SENSORS
	for i in 400:
		bs.step(0.05)
	check(bs.mech.camera_ok(3), "drones at the sensor head rewire the LEFT camera (%.0f)" % bs.mech.cameras[3])

	print("== campaign save round-trip")
	var cs := Campaign.new()
	cs.mission = 3
	cs.install(Campaign.Part.NAVY_RADIATOR)
	cs.max_hp[Rules.Section.LEFT_LEG] = 61.0
	cs.matrix[Rules.Sys.LOCO] = true
	cs.last_report = {"number": 2, "time": 250.0, "kills": 8, "heat_peak": 90.0, "scars": [[Rules.Section.LEFT_LEG, 9.0]]}
	cs.roll_salvage(Rng.new(5))
	var back := Campaign.new()
	back.from_dict(JSON.parse_string(JSON.stringify(cs.to_dict())))
	check(back.mission == 3 and back.installed == cs.installed and back.max_hp == cs.max_hp and back.matrix == cs.matrix and back.salvage == cs.salvage, "campaign survives JSON")
	check(absf(back.cooling_mult - cs.cooling_mult) < 0.001 and back.last_report.scars[0][0] == Rules.Section.LEFT_LEG, "modifiers and report survive JSON")

	print(("ALL PASSED" if failures == 0 else "%d FAILURES" % failures))
	quit(0 if failures == 0 else 1)
