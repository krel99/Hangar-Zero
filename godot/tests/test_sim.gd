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

	print(("ALL PASSED" if failures == 0 else "%d FAILURES" % failures))
	quit(0 if failures == 0 else 1)
