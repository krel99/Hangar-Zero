## The whole game in one Control: owns the campaign, runs the sim, paints the active
## screen and routes touch / gyro / keys. `HZ_SCENE` stages a screenshot (see Shots and
## scripts/shots.sh); `HZ_SCENE=play` hands the mech to the autopilot and saves a strip.
extends Control

enum Screen { COCKPIT, HANGAR }

var campaign := Campaign.new()
var rng := Rng.new(int(Time.get_unix_time_from_system()) & 0x7FFFFFFF)
var sim: Sim
var screen: int = Screen.COCKPIT
var cockpit := Cockpit.new()
var hangar := HangarView.new()
var t := 0.0

## Screenshot plan from the environment; empty scene = normal play.
var shot_scene := OS.get_environment("HZ_SCENE")
var shot_path := OS.get_environment("HZ_SHOT")
var _shooting := false


func _ready() -> void:
	set_process_input(true)
	if shot_scene != "":
		_shooting = true
		rng = Rng.new(int(OS.get_environment("HZ_SEED")) if OS.get_environment("HZ_SEED") != "" else 1987)
		_run_shots.call_deferred()
		return
	_deploy()


func _deploy() -> void:
	sim = Sim.new(campaign, rng.index(1 << 30))
	cockpit.set_sim(sim)
	screen = Screen.COCKPIT


func _to_hangar() -> void:
	campaign.apply_scars(sim.mission.number, sim.mission.time, sim.mission.damage_taken, sim.mech.kills, sim.mech.heat_peak)
	campaign.cameras.assign(sim.mech.cameras)
	campaign.mission += 1
	campaign.roll_salvage(rng)
	hangar.set_campaign(campaign)
	screen = Screen.HANGAR


func _new_campaign() -> void:
	campaign = Campaign.new()
	_deploy()


func _process(dt: float) -> void:
	t += dt
	if _shooting:
		queue_redraw()
		return
	match screen:
		Screen.COCKPIT:
			cockpit.gyro_rate = _gyro_yaw()
			cockpit.update(dt)
			sim.step(dt)
			if cockpit.request == "hangar":
				_to_hangar()
			elif cockpit.request == "new_campaign":
				_new_campaign()
		Screen.HANGAR:
			if hangar.request == "deploy":
				_deploy()
	queue_redraw()


## Rotation rate about the gravity axis: turning the phone left or right, however it is
## held. Positive is clockwise seen from above, which is a bearing increase.
func _gyro_yaw() -> float:
	var g := Input.get_gravity()
	if g.length_squared() < 1.0:
		return 0.0
	return Input.get_gyroscope().dot(g.normalized())


func _draw() -> void:
	if sim == null and screen == Screen.COCKPIT:
		return
	match screen:
		Screen.COCKPIT:
			cockpit.draw(self, t)
		Screen.HANGAR:
			hangar.draw(self, t)


func _input(event: InputEvent) -> void:
	if _shooting:
		return
	if event is InputEventScreenTouch:
		if event.pressed:
			if screen == Screen.COCKPIT:
				cockpit.press(event.position)
		else:
			if screen == Screen.COCKPIT:
				cockpit.release(event.position)
			else:
				hangar.tap(event.position)
	elif event is InputEventScreenDrag:
		if screen == Screen.COCKPIT:
			cockpit.drag(event.position, event.relative)
	elif event is InputEventKey and event.pressed and not event.echo:
		if screen == Screen.COCKPIT:
			cockpit.key(event.keycode, event.shift_pressed)
		else:
			hangar.key(event.keycode)
		get_viewport().set_input_as_handled()


# ---------------------------------------------------------------------------------------
# Screenshots
# ---------------------------------------------------------------------------------------

func _env_f(name: String, dflt: float) -> float:
	var v := OS.get_environment(name)
	return float(v) if v != "" else dflt


func _run_shots() -> void:
	await get_tree().process_frame
	if shot_path == "":
		shot_path = shot_scene + ".png"
	if shot_scene == "play":
		var frames := int(_env_f("HZ_FRAMES", 8))
		var every := _env_f("HZ_EVERY", 25)
		_deploy()
		sim.mission.briefing = false
		for i in frames:
			while sim.mission.time < i * every and sim.mission.outcome == Rules.Outcome.NONE:
				Autopilot.act(sim)
				sim.step(0.05)
			t = sim.mission.time
			await _capture("%s_%02d.png" % [shot_path.trim_suffix(".png"), i])
			print("captured frame %d at T+%d" % [i, int(sim.mission.time)])
			if sim.mission.outcome != Rules.Outcome.NONE:
				break
		get_tree().quit()
		return
	if shot_scene == "hangar":
		Shots.stage_hangar(campaign, rng)
		hangar.set_campaign(campaign)
		screen = Screen.HANGAR
	else:
		_deploy()
		if shot_scene != "briefing":
			Shots.stage_combat(shot_scene, sim)
		var steps := int(Shots.settle(shot_scene) * 60.0)
		for i in steps:
			sim.step(1.0 / 60.0)
	t = Shots.settle(shot_scene)
	await _capture(shot_path)
	print("captured ", shot_path)
	get_tree().quit()


func _capture(path: String) -> void:
	queue_redraw()
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	if img.get_width() < 1440:
		img.resize(img.get_width() * 3, img.get_height() * 3, Image.INTERPOLATE_NEAREST)
	img.save_png(path)
