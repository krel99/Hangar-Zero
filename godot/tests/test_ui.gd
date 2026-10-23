## Drives the real main scene with synthetic touches and keys under a display (xvfb-run)
## and checks the input reaches the sim: station taps, feed drag, radar tap, hangar flow.
extends SceneTree

var fails := 0


func check(cond: bool, what: String) -> void:
	if not cond:
		fails += 1
		printerr("FAIL: ", what)
	else:
		printerr("ok: ", what)


func touch(p: Vector2, pressed: bool) -> void:
	var ev := InputEventScreenTouch.new()
	ev.position = p
	ev.pressed = pressed
	ev.index = 0
	Input.parse_input_event(ev)


func drag(from: Vector2, to: Vector2, steps: int = 6) -> void:
	touch(from, true)
	await process_frame
	for i in range(1, steps + 1):
		var ev := InputEventScreenDrag.new()
		ev.position = from.lerp(to, float(i) / steps)
		ev.relative = (to - from) / steps
		ev.index = 0
		Input.parse_input_event(ev)
		await process_frame
	touch(to, false)
	await process_frame


func tap(p: Vector2) -> void:
	touch(p, true)
	await process_frame
	touch(p, false)
	await process_frame
	await process_frame


func key(code: Key) -> void:
	var ev := InputEventKey.new()
	ev.keycode = code
	ev.pressed = true
	Input.parse_input_event(ev)
	await process_frame
	ev = InputEventKey.new()
	ev.keycode = code
	ev.pressed = false
	Input.parse_input_event(ev)
	await process_frame


func _init() -> void:
	call_deferred("_run")


func _run() -> void:
	Campaign.clear_saved()
	var main: Control = load("res://ui/main.tscn").instantiate()
	root.add_child(main)
	await process_frame
	await process_frame
	var sim: Sim = main.sim
	check(sim != null, "sim created on boot")
	check(sim.mission.briefing, "briefing shown first")
	# Window coordinates are scaled by the stretch; the viewport is 480x270.
	var scale := Vector2(root.size) / Vector2(480, 270)
	var to_win := func(p: Vector2) -> Vector2: return p * scale
	await tap(to_win.call(Vector2(240, 221)))  # BEGIN OPERATION (bottom of the briefing modal)
	check(not sim.mission.briefing, "tap dismisses the briefing")
	# VENT station (4th slot) sets the vent timer
	await tap(to_win.call(Vector2(444, 3.5 * 42)))
	await process_frame
	check(sim.mech.vent_t > 0.0, "tap on VENT station vents (vent_t=%.1f)" % sim.mech.vent_t)
	# A/C toggle
	var wf := sim.mech.weapons_free
	await tap(to_win.call(Vector2(444, 1.5 * 42)))
	check(sim.mech.weapons_free != wf, "tap on A/C toggles weapons free")
	# Dragging the feed turns the view
	var v0 := sim.mech.view
	await drag(to_win.call(Vector2(300, 100)), to_win.call(Vector2(200, 100)))
	var turned := Rules.angle_diff(sim.mech.view, v0)
	check(turned > 0.3, "drag of 100px turns the view by %.2f rad" % turned)
	for i in 60:
		await process_frame
	check(Rules.camera_for(sim.mech.view) == 1 and absf(Rules.angle_diff(sim.mech.view, Rules.camera_bearing(1))) < 0.01, "a flick settles on the RIGHT camera")
	turned = Rules.angle_diff(sim.mech.view, v0)
	# Tab snaps to the next camera
	await key(KEY_TAB)
	for i in 60:
		await process_frame
	var snapped := false
	for cam in Rules.CAMERAS:
		if absf(Rules.angle_diff(sim.mech.view, Rules.camera_bearing(cam))) < 0.001:
			snapped = true
	check(snapped and Rules.camera_for(sim.mech.view) != Rules.camera_for(v0 + turned), "TAB snaps to the next camera")
	# Radar tap targets the blip under the finger
	var e := sim.spawn_enemy(Rules.Kind.SCOUT, deg_to_rad(90.0), 300.0)
	e.est_angle = e.angle
	e.est_dist = e.dist
	sim.mech.target = -1
	var blip := Vector2(44, 44) + Vector2(sin(e.angle), -cos(e.angle)) * (e.dist * 35.0 / Rules.RADAR_SCALE_M)
	await tap(to_win.call(blip))
	check(sim.mech.target == e.id, "radar tap selects the contact")
	# A second tap on the locked contact fires the rail
	var e2 := sim.spawn_enemy(Rules.Kind.ARTILLERY, sim.mech.view, 300.0)
	e2.covered = true
	sim.mech.rail_charge = 1.0
	sim.mech.heat = 30.0
	await process_frame
	var fx: Vector2 = to_win.call(Vector2(88 + 160, 120))
	await tap(fx)
	check(sim.mech.target == e2.id, "feed tap locks the contact in view")
	await tap(fx)
	check(sim.mech.rail_charge < 0.5, "second tap on the contact fires the rail (charge %.2f)" % sim.mech.rail_charge)
	# Hull tap moves the repair drones
	await tap(to_win.call(Vector2(4 + 20 * 2, 92 + 20 * 2)))  # reactor
	check(sim.mech.repair_target == Rules.Section.REACTOR, "hull tap sends drones to the reactor")
	# PURGE station clears boarders
	var bd := Entities.Boarder.new()
	bd.section = Rules.Section.COOLANT
	sim.boarders.append(bd)
	await process_frame
	await tap(to_win.call(Vector2(444, 5.5 * 42)))
	check(sim.boarders.is_empty() and sim.mech.purge_cd > 0.0, "tap on PURGE fries the boarder")
	# EJECT lives on the overload banner
	sim.mech.overload = 5.0
	sim.mech.heat = 110.0
	await process_frame
	await process_frame
	check(main.cockpit.feed.eject_btn.size.x > 0.0, "overload shows the EJECT button")
	sim.mech.overload = -1.0
	sim.mech.heat = 40.0
	# Pause and resume
	await tap(to_win.call(Vector2(444, 261)))
	check(sim.mission.paused, "PAUSE button pauses")
	await key(KEY_P)
	check(not sim.mission.paused, "P resumes")
	# Victory -> hangar -> deploy
	sim.mech.hp[Rules.Section.UPLINK] = 100.0
	await process_frame
	await process_frame
	check(sim.mission.outcome == Rules.Outcome.VICTORY, "uplink at 100 wins")
	await tap(to_win.call(Vector2(292, 212)))  # RETURN TO HANGAR
	check(main.screen == 1, "outcome button goes to the hangar")
	check(main.campaign.mission == 2, "mission counter advanced")
	check(main.campaign.salvage.size() == 5, "five salvage choices")
	var h: HangarView = main.hangar
	await process_frame
	var btn: Rect2 = h._install_btns[1]
	await tap(to_win.call(btn.get_center()))
	check(h.chosen == main.campaign.salvage[1], "install button picks a part")
	await tap(to_win.call(HangarView.DEPLOY_BTN.get_center()))
	check(main.screen == 0 and main.sim != sim, "deploy starts op 2")
	check(main.sim.mission.number == 2, "op 2 mission number")
	# Defeat offers a retry of the same op
	var s2: Sim = main.sim
	s2.mission.briefing = false
	s2.mission.outcome = Rules.Outcome.MELTDOWN
	await process_frame
	await process_frame
	await tap(to_win.call(Vector2(234, 212)))  # RETRY OP
	check(main.screen == 0 and main.sim != s2 and main.sim.mission.number == 2, "RETRY replays op 2 with the same campaign")
	if fails == 0:
		printerr("ALL PASSED")
	else:
		printerr("%d FAILED" % fails)
	quit(1 if fails > 0 else 0)
