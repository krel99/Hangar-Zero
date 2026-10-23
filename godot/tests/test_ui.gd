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
	await tap(to_win.call(Vector2(444, 3.5 * 36)))
	await process_frame
	check(sim.mech.vent_t > 0.0, "tap on VENT station vents (vent_t=%.1f)" % sim.mech.vent_t)
	# A/C toggle
	var wf := sim.mech.weapons_free
	await tap(to_win.call(Vector2(444, 1.5 * 36)))
	check(sim.mech.weapons_free != wf, "tap on A/C toggles weapons free")
	# Dragging the feed turns the view
	var v0 := sim.mech.view
	await drag(to_win.call(Vector2(300, 100)), to_win.call(Vector2(200, 100)))
	var turned := Rules.angle_diff(sim.mech.view, v0)
	check(absf(turned - Rules.CAMERA_FOV * 100.0 / 320.0) < 0.05, "drag of 100px turns the view by %.2f rad" % turned)
	# Tab snaps to the next camera
	await key(KEY_TAB)
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
	# Hull tap moves the repair drones
	await tap(to_win.call(Vector2(4 + 20 * 2, 92 + 20 * 2)))  # reactor
	check(sim.mech.repair_target == Rules.Section.REACTOR, "hull tap sends drones to the reactor")
	# PURGE station clears boarders
	var bd := Entities.Boarder.new()
	bd.section = Rules.Section.COOLANT
	sim.boarders.append(bd)
	await process_frame
	await tap(to_win.call(Vector2(444, 6.5 * 36)))
	check(sim.boarders.is_empty() and sim.mech.purge_cd > 0.0, "tap on PURGE fries the boarder")
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
	if fails == 0:
		printerr("ALL PASSED")
	else:
		printerr("%d FAILED" % fails)
	quit(1 if fails > 0 else 0)
