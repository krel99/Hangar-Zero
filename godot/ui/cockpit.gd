## The cockpit on a landscape phone (480x270): radar and hull on the left, the camera
## feed in the middle, the seven stations down the right. Drag or turn the phone to look
## around; tap a blip, a hull section or a station.
class_name Cockpit
extends RefCounted

const RADAR := Rect2(0, 0, 88, 88)
const HULL_BOX := Rect2(0, 90, 88, 96)
const HULL_K := 2.0
const HULL_ORIGIN := Vector2(4, 92)
const FEED_POS := Vector2(88, 0)
const STATION_X := 408.0
const STATION_W := 72.0
const STATION_H := 42.0
const PAUSE_BTN := Rect2(410, 254, 68, 14)
const UPLINK_BAR := Rect2(88, 210, 252, 6)
const RADIO := Rect2(88, 219, 320, 51)

const ICON_SCOUT := ["...x...", "..xxx..", ".xx.xx.", "xx...xx", ".xx.xx.", "..xxx..", "...x..."]
const ICON_ART := ["xxxxxxx", "x.....x", "x.xxx.x", "x.xxx.x", "x.xxx.x", "x.....x", "xxxxxxx"]
const ICON_CARRIER := ["..xxx..", ".x...x.", "x.x.x.x", "x..x..x", "x.x.x.x", ".x...x.", "..xxx.."]
const ICON_UNKNOWN := ["...x...", "...x...", ".......", "xx.x.xx", ".......", "...x...", "...x..."]
const SELF_ICON := ["....x....", "....x....", ".xx.x.xx.", "xxxxxxxxx", "xxxxxxxxx", "xxxxxxxxx", ".xx...xx.", "....xxxx.", "......xx."]

const VOICE_NAME := ["CONTROL", "ODA", "YOU", "INTERCEPT", "OS"]

var sim: Sim
var feed := Feed.new()
var t := 0.0
## Set by the outcome modal: "hangar" or "new_campaign".
var request := ""
## Set from Input.get_gyroscope() each frame by the owner; rad/s about gravity.
var gyro_rate := 0.0

var _press := Vector2.ZERO
var _press_in_feed := false
## Horizontal drag since the press, in feed pixels (for the flick-to-next-camera gesture).
var _drag_dx := 0.0
var _press_cam := 0
## Seconds since the view was last moved by the player; the view settles onto a whole
## camera once the hand is still, so the feed is never split between two cameras.
var _still_t := 10.0
var _snap := -1.0
var _dragged := 0.0
var _down_station := -1
var _modal_btn := Rect2()
var _gyro_btn := Rect2()
var _modal_btn2 := Rect2()
var _station_rects: Array[Rect2] = []
var _hull_centroid: Array[Vector2] = []
## The hull body is baked into a small image a dozen times a second instead of ~4000
## rect draws per frame; the overlays (drone outline, ticks, shells) stay live.
const HULL_PAD := 2
var _hull_img := Image.create(Hull.W + HULL_PAD * 2, Hull.H + HULL_PAD * 2, false, Image.FORMAT_RGBA8)
var _hull_tex: ImageTexture
var _hull_next := -1.0
## Static per-cell data: halo distance (0 none, 1, 2) over the padded grid, edge/hi flags.
var _halo := PackedByteArray()
var _edge := PackedByteArray()
var _hi := PackedByteArray()
var _last_shake := 0.0
var _last_kills := 0
var _last_overload := false
## Settings (persisted in user://settings.cfg).
static var gyro_enabled := true
## First-op hints: 0 swipe, 1 tap a contact, 2 fire the rail, 3 done.
static var hint_stage := 3
var _hint_turned := 0.0


func _init() -> void:
	for i in 6:
		_station_rects.append(Rect2(STATION_X, i * STATION_H, STATION_W, STATION_H))
	_hull_static()
	_hull_tex = ImageTexture.create_from_image(_hull_img)
	load_settings()


static func load_settings() -> void:
	var cf := ConfigFile.new()
	if cf.load("user://settings.cfg") == OK:
		gyro_enabled = cf.get_value("input", "gyro", true)
		hint_stage = cf.get_value("help", "stage", 0)
	else:
		hint_stage = 0


static func save_settings() -> void:
	var cf := ConfigFile.new()
	cf.set_value("input", "gyro", gyro_enabled)
	cf.set_value("help", "stage", hint_stage)
	cf.save("user://settings.cfg")


func _hull_static() -> void:
	var pw := Hull.W + HULL_PAD * 2
	var ph := Hull.H + HULL_PAD * 2
	_halo.resize(pw * ph)
	_halo.fill(0)
	for py in ph:
		for px in pw:
			var x := px - HULL_PAD
			var y := py - HULL_PAD
			if Hull.at(x, y) != Hull.EMPTY:
				continue
			var best := 3
			for dy in range(-2, 3):
				for dx in range(-2, 3):
					if Hull.at(x + dx, y + dy) != Hull.EMPTY:
						best = mini(best, maxi(absi(dx), absi(dy)))
			if best <= 2:
				_halo[py * pw + px] = best
	_edge.resize(Hull.W * Hull.H)
	_hi.resize(Hull.W * Hull.H)
	var cen_sum: Array = []
	var cen_n: Array = []
	for sec in Rules.SECTIONS:
		cen_sum.append(Vector2.ZERO)
		cen_n.append(0)
	for y in Hull.H:
		for x in Hull.W:
			var v := Hull.at(x, y)
			if v == Hull.EMPTY:
				continue
			_edge[y * Hull.W + x] = 1 if (Hull.at(x - 1, y) != v or Hull.at(x + 1, y) != v or Hull.at(x, y - 1) != v or Hull.at(x, y + 1) != v) else 0
			_hi[y * Hull.W + x] = 1 if (Hull.at(x, y - 1) != v or Hull.at(x - 1, y) != v) else 0
			if v < Rules.SECTIONS:
				cen_sum[v] += Vector2(x, y)
				cen_n[v] += 1
	_hull_centroid.clear()
	for sec in Rules.SECTIONS:
		var n: int = maxi(cen_n[sec], 1)
		_hull_centroid.append(HULL_ORIGIN + (cen_sum[sec] / n + Vector2(0.5, 0.5)) * HULL_K)


func _hull_bake(boarders_on: Array) -> void:
	var mech := sim.mech
	var st: Dictionary = mech.st
	var img := _hull_img
	img.fill(Color(0, 0, 0, 0))
	var pw := Hull.W + HULL_PAD * 2
	var sfrac: float = clampf(mech.shield / st.shield_cap, 0.0, 1.0) if st.shield_cap > 0.0 else 0.0
	var down := mech.shields_down_t > 0.0
	if sfrac > 0.02 or down:
		var halo := Theme2.RED if down else Theme2.SHIELD
		var blink := fmod(t * 8.0, 1.0) < 0.5
		for i in _halo.size():
			var best := _halo[i]
			if best == 0:
				continue
			var px := i % pw
			var py := i / pw
			var shimmer := sin((px + py) * 0.7 + t * 6.0) * 0.5 + 0.5
			var base := 0.7 if best == 1 else 0.3
			var a: float = (0.4 if blink else 0.0) if down else base * (0.4 + 0.6 * sfrac) * (0.7 + 0.3 * shimmer)
			if a > 0.0:
				img.set_pixel(px, py, Color(halo, a * 0.6))
	var scan_y := int(fmod(t * 0.35, 1.0) * (Hull.H + 6)) - 3
	var dead_blink := fmod(t * 2.0, 1.0) < 0.5
	var purple := (sin(t * 3.0) * 0.5 + 0.5) * 0.6
	for y in Hull.H:
		for x in Hull.W:
			var v := Hull.at(x, y)
			if v == Hull.EMPTY:
				continue
			if Hull.at(x + 1, y + 1) == Hull.EMPTY:
				img.set_pixel(x + 1 + HULL_PAD, y + 1 + HULL_PAD, Color(0, 0, 0, 0.47))
	for y in Hull.H:
		for x in Hull.W:
			var v := Hull.at(x, y)
			if v == Hull.EMPTY:
				continue
			var col: Color
			if v == Hull.FRAME:
				col = Color8(54, 60, 68)
			else:
				var f: float = mech.uplink() / 100.0 if v == Rules.Section.UPLINK else mech.frac(v)
				if v == Rules.Section.UPLINK:
					col = Theme2.CYAN if y / 7.0 > 1.0 - f else Color8(30, 60, 70)
				else:
					col = Hull.health_color(f)
				if f <= 0.0 and v != Rules.Section.UPLINK and dead_blink:
					col = Color8(20, 10, 10)
				if v != Rules.Section.UPLINK and f < 0.6 and Rng.hash2(x, y, 4) % 7 == 0:
					col = Theme2.dim(col, 0.4)
				if boarders_on[v] > 0:
					col = col.lerp(Theme2.PURPLE, purple)
				var flash: float = mech.hit_t[v]
				if flash > 0.0:
					col = col.lerp(Color8(255, 240, 200), flash * flash)
			var d := Hull.detail_at(x, y)
			var i := y * Hull.W + x
			if d == Hull.D.HAZARD:
				col = Color8(24, 22, 20) if Hull.hazard_dark(x, y) else Hull.shade(col, d, t)
			elif d != Hull.D.NONE:
				col = Hull.shade(col, d, t)
			elif _edge[i] == 1:
				col = Theme2.dim(col, 1.15) if _hi[i] == 1 else Theme2.dim(col, 0.45)
			else:
				col = Theme2.dim(col, 0.78)
			if y == scan_y:
				col = Theme2.dim(col, 1.6)
			col.a = 1.0
			img.set_pixel(x + HULL_PAD, y + HULL_PAD, col)
	_hull_tex.update(img)


func set_sim(s: Sim) -> void:
	sim = s
	feed.set_sim(s)
	feed.origin = FEED_POS
	request = ""


func modal_open() -> bool:
	return sim.mission.briefing or sim.mission.paused or sim.mission.outcome != Rules.Outcome.NONE


# ---------------------------------------------------------------------------------------
# Per-frame
# ---------------------------------------------------------------------------------------

func update(dt: float) -> void:
	var mech := sim.mech
	_haptics(mech)
	if not sim.mission.running():
		return
	if gyro_enabled and absf(gyro_rate) > 0.02:
		mech.view = fposmod(mech.view + gyro_rate * dt, TAU)
		_hint_turned += absf(gyro_rate * dt)
		_still_t = 0.0
		_snap = -1.0
	elif not _press_in_feed:
		_still_t += dt
	if _press_in_feed:
		return
	# Settle: a flick picked a camera; otherwise the nearest one after a short pause.
	var target := _snap
	if target < 0.0 and _still_t > 0.45:
		target = Rules.camera_bearing(Rules.camera_for(mech.view))
	if target >= 0.0:
		var d := Rules.angle_diff(target, mech.view)
		if absf(d) < 0.004:
			mech.view = target
			_snap = -1.0
		else:
			mech.view = fposmod(mech.view + d * minf(dt * 9.0, 1.0), TAU)


## Short buzzes for the things the pilot would feel: a rail shot, a shell, a kill, the
## overload alarm. No-op without a vibrator.
func _haptics(mech: Mech) -> void:
	if mech.shake > _last_shake + 0.3:
		Input.vibrate_handheld(70 if mech.shake > 1.2 else 30)
	_last_shake = mech.shake
	if mech.kills > _last_kills:
		Input.vibrate_handheld(20)
	_last_kills = mech.kills
	var overload := mech.overload >= 0.0
	if overload and not _last_overload:
		Input.vibrate_handheld(200)
	_last_overload = overload


func draw(ci: CanvasItem, now: float) -> void:
	t = now
	ci.draw_rect(Rect2(0, 0, 480, 270), Theme2.BG)
	_radar(ci)
	_hull(ci)
	feed.draw(ci, now)
	_hints(ci)
	_uplink(ci)
	_radio(ci)
	_stations(ci)
	Theme2.button(ci, PAUSE_BTN, "PAUSE", Theme2.DIM, true, false)
	_modals(ci)


# ---------------------------------------------------------------------------------------
# Radar
# ---------------------------------------------------------------------------------------

func _radar(ci: CanvasItem) -> void:
	var mech := sim.mech
	var st: Dictionary = mech.st
	var square: bool = sim.campaign.installed.has(Campaign.Part.JACKAL_DISH)
	var c := Vector2(44, 44)
	var rad := 35.0
	var k := rad / Rules.RADAR_SCALE_M
	var range_px: float = minf(st.radar_range * k, rad)
	ci.draw_rect(RADAR, Theme2.PANEL_DARK)
	var scope_bg := Color8(6, 16, 10)
	var lit_bg := Color8(10, 30, 16)
	if square:
		ci.draw_rect(Rect2(c - Vector2(rad, rad), Vector2(rad * 2, rad * 2)), scope_bg)
		ci.draw_rect(Rect2(c - Vector2(range_px, range_px), Vector2(range_px * 2, range_px * 2)), lit_bg)
		var g := -rad
		while g <= rad:
			ci.draw_line(c + Vector2(g, -rad), c + Vector2(g, rad), Theme2.dim(Theme2.GREEN_DIM, 0.6))
			ci.draw_line(c + Vector2(-rad, g), c + Vector2(rad, g), Theme2.dim(Theme2.GREEN_DIM, 0.6))
			g += rad / 4.0
		ci.draw_rect(Rect2(c - Vector2(rad, rad), Vector2(rad * 2, rad * 2)), Campaign.MAKER_ACCENT[Campaign.Maker.JACKAL], false, 1.0)
	else:
		ci.draw_circle(c, rad, scope_bg)
		ci.draw_circle(c, range_px, lit_bg)
		for i in range(1, 4):
			Theme2.ring(ci, c, rad * i / 4.0, Theme2.GREEN_DIM)
		ci.draw_line(c - Vector2(rad, 0), c + Vector2(rad, 0), Theme2.GREEN_DIM)
		ci.draw_line(c - Vector2(0, rad), c + Vector2(0, rad), Theme2.GREEN_DIM)
	# Ground clutter the sweep lights up
	for i in 60:
		var h := Rng.hash2(i, 0, 61)
		var a := (h % 6283) / 1000.0
		var d: float = 120.0 + (h >> 13) % 700
		if d > st.radar_range:
			continue
		var since := fposmod(mech.sweep - a, TAU)
		var glow := 0.12 + 0.25 * (1.0 - since / TAU)
		var p := Theme2.polar(c, a, d * k)
		ci.draw_rect(Rect2(p.round(), Vector2(1 + (h >> 24) % 2, 1)), Color(Theme2.GREEN, glow))
	Theme2.ring(ci, c, range_px, Color(Theme2.GREEN, 0.7), true)
	Theme2.ring(ci, c, minf(st.id_range * k, rad), Color(Theme2.GREEN, 0.3), true)
	Theme2.ring(ci, c, Rules.AC_RANGE * k, Color(Theme2.YELLOW, 0.25), true)
	for i in 4:
		var lbl: String = ["N", "E", "S", "W"][i]
		var p := Theme2.polar(c, i * PI / 2.0, rad + 5.0)
		Theme2.label(ci, p + Vector2(0, 3), lbl, Color(Theme2.GREEN, 0.7), 8, 1)
	# Sweep with phosphor trail
	for i in 16:
		var a := mech.sweep - i * 0.03
		ci.draw_line(c, Theme2.polar(c, a, range_px), Color(Theme2.GREEN, (1.0 - i / 16.0) * 0.5))
	# Sensor damage: static and ghosts
	var sf := mech.frac(Rules.Section.SENSORS)
	var frame := int(t * 12.0)
	for i in int((1.0 - sf) * 90.0):
		var h := Rng.hash2(i, frame, 5)
		var a := (h % 6283) / 1000.0
		var d: float = ((h >> 13) % 1000) / 1000.0 * st.radar_range
		ci.draw_rect(Rect2(Theme2.polar(c, a, d * k).round(), Vector2.ONE), Color(Theme2.GREEN, 0.35))
	if sf < 0.5 or square:
		var epoch := int(t / 3.0)
		for g in 2:
			var h := Rng.hash2(g, epoch, 99)
			var a := (h % 6283) / 1000.0
			var d: float = 150.0 + (h >> 12) % 500
			if d < st.radar_range:
				Theme2.icon7(ci, Theme2.polar(c, a, d * k), ICON_UNKNOWN, 1.0, Color(Theme2.AMBER, 0.5))
	# Quadrants with no working camera are radar-only
	for cam in Rules.CAMERAS:
		if mech.camera_ok(cam):
			continue
		var a0 := Rules.camera_bearing(cam) - Rules.CAMERA_FOV / 2.0
		var pts := PackedVector2Array([c])
		for i in 13:
			pts.append(Theme2.polar(c, a0 + Rules.CAMERA_FOV * i / 12.0, rad))
		ci.draw_colored_polygon(pts, Color8(120, 30, 30, 60))
		if fmod(t * 1.2, 1.0) < 0.7:
			var lp := Theme2.polar(c, Rules.camera_bearing(cam), rad * 0.6)
			Theme2.label(ci, lp + Vector2(0, -1), "NO", Color(Theme2.RED, 0.8), 8, 1)
			Theme2.label(ci, lp + Vector2(0, 8), "CAM", Color(Theme2.RED, 0.8), 8, 1)
	# Where the pilot is looking
	var va := mech.view
	var cone := PackedVector2Array([c, Theme2.polar(c, va - Rules.CAMERA_FOV / 2.0, rad), Theme2.polar(c, va, rad * 1.08), Theme2.polar(c, va + Rules.CAMERA_FOV / 2.0, rad)])
	ci.draw_colored_polygon(cone, Color(Theme2.CYAN, 0.08))
	ci.draw_line(c, Theme2.polar(c, va, rad), Color(Theme2.CYAN, 0.35))
	# Incoming shells
	for s in sim.shells:
		var f: float = clampf(s.t / s.flight, 0.0, 1.0)
		var from := Theme2.polar(c, s.angle, rad)
		var at := from + (c - from) * f
		ci.draw_line(from, at, Color(Theme2.RED, 0.6))
		ci.draw_rect(Rect2(at - Vector2(1.5, 1.5), Vector2(3, 3)), Theme2.RED)
	# Contacts
	for e in sim.enemies:
		var pos := Theme2.polar(c, e.est_angle, minf(e.est_dist, Rules.RADAR_SCALE_M) * k)
		var since := fposmod(mech.sweep - e.est_angle, TAU)
		var glow := 1.0 - since / TAU * 0.65
		if e.dist > st.radar_range:
			if e.reveal > 0.0:
				var edge := Theme2.polar(c, e.angle, rad)
				ci.draw_line(c + (edge - c) * 0.75, edge, Color8(255, 140, 60), 2.0)
			continue
		var identified: bool = e.dist <= st.id_range or e.covered
		var dimmer := 1.0 if e.covered else 0.7
		if identified:
			var col := Theme2.dim(Theme2.kind_color(e.kind), glow * dimmer)
			var icon: Array = [ICON_SCOUT, ICON_ART, ICON_CARRIER][e.kind]
			Theme2.icon7(ci, pos, icon, 1.0, Color.WHITE if e.hit_flash > 0.0 else col)
		else:
			Theme2.icon7(ci, pos, ICON_UNKNOWN, 1.0, Theme2.dim(Theme2.AMBER, glow * dimmer))
		if not e.covered:
			Theme2.ring(ci, pos, maxf(e.err * e.est_dist * k, 3.0), Color(Theme2.AMBER, 0.45), true)
		if e.id == mech.target:
			if fmod(t * 4.0, 1.0) < 0.7:
				var b := 6.0
				for sx in [-1.0, 1.0]:
					for sy in [-1.0, 1.0]:
						var corner := pos + Vector2(sx * b, sy * b)
						ci.draw_line(corner, corner - Vector2(sx * 3.0, 0), Color.WHITE)
						ci.draw_line(corner, corner - Vector2(0, sy * 3.0), Color.WHITE)
			Theme2.text(ci, pos + Vector2(8, -6), e.callsign(), Color.WHITE, 12)
	for d in sim.drones:
		if d.dist <= st.radar_range:
			ci.draw_rect(Rect2(Theme2.polar(c, d.angle, d.dist * k) - Vector2(1, 1), Vector2(2, 2)), Theme2.PURPLE)
	# Self + shield bubble
	if mech.shield > 1.0:
		var sfrac: float = mech.shield / st.shield_cap if st.shield_cap > 0.0 else 0.0
		Theme2.ring(ci, c, 7.0, Color(Theme2.SHIELD, 0.3 + 0.7 * sfrac))
	Theme2.icon7(ci, c, SELF_ICON, 1.0, Theme2.AMBER)
	Theme2.text(ci, Vector2(3, 86), "%dm" % int(st.radar_range), Theme2.GREEN, 12)
	if square:
		Theme2.text(ci, Vector2(85, 12), "JKL", Campaign.MAKER_ACCENT[Campaign.Maker.JACKAL], 10, 2)


# ---------------------------------------------------------------------------------------
# Hull diagram and core gauges
# ---------------------------------------------------------------------------------------

func _cell(x: int, y: int) -> Rect2:
	return Rect2(HULL_ORIGIN + Vector2(x, y) * HULL_K, Vector2(HULL_K, HULL_K))


func _hull(ci: CanvasItem) -> void:
	var mech := sim.mech
	var st: Dictionary = mech.st
	ci.draw_rect(HULL_BOX, Color8(8, 14, 18))
	var gx := HULL_BOX.position.x
	while gx < HULL_BOX.end.x:
		ci.draw_rect(Rect2(gx, HULL_BOX.position.y, 1, HULL_BOX.size.y), Color8(16, 30, 38))
		gx += 8.0
	var gy := HULL_BOX.position.y
	while gy < HULL_BOX.end.y:
		ci.draw_rect(Rect2(HULL_BOX.position.x, gy, HULL_BOX.size.x, 1), Color8(16, 30, 38))
		gy += 8.0
	var sfrac: float = clampf(mech.shield / st.shield_cap, 0.0, 1.0) if st.shield_cap > 0.0 else 0.0
	var down := mech.shields_down_t > 0.0
	var boarders_on: Array = []
	boarders_on.resize(Rules.SECTIONS)
	boarders_on.fill(0)
	for b in sim.boarders:
		boarders_on[b.section] += 1
	if t >= _hull_next or mech.hit_t.max() > 0.9:
		_hull_bake(boarders_on)
		_hull_next = t + 0.08
	ci.draw_texture_rect(_hull_tex, Rect2(HULL_ORIGIN - Vector2(HULL_PAD, HULL_PAD) * HULL_K, Vector2(_hull_img.get_width(), _hull_img.get_height()) * HULL_K), false)
	# Repair drones: blinking outline on their target
	var rt := mech.repair_target
	if fmod(t * 2.0, 1.0) < 0.6:
		for y in Hull.H:
			for x in Hull.W:
				if Hull.at(x, y) == rt and _edge[y * Hull.W + x] == 1:
					ci.draw_rect(_cell(x, y), Color8(240, 130, 60), false, 1.0)
	# Boarders and incoming shells
	for s in Rules.SECTIONS:
		var n: int = boarders_on[s]
		for i in n:
			var off := Vector2((i - (n - 1) / 2.0) * 10.0, 0)
			var wig := roundf(sin(t * 6.0 + i))
			Pixel.draw(ci, Sprites.TICK, _hull_centroid[s] + off - Vector2(6, 5 + wig), 1.0, i % 2 == 1)
	for sh in sim.shells:
		var cc: Vector2 = _hull_centroid[sh.target]
		var rr: float = 3.0 + (1.0 - sh.t / sh.flight) * 8.0
		if fmod(t * 6.0, 1.0) < 0.6:
			Theme2.ring(ci, cc, rr, Theme2.RED)
			ci.draw_line(cc - Vector2(rr + 2, 0), cc + Vector2(rr + 2, 0), Theme2.RED)
			ci.draw_line(cc - Vector2(0, rr + 2), cc + Vector2(0, rr + 2), Theme2.RED)
	if mech.purge_t > 0.0:
		ci.draw_rect(HULL_BOX, Color(0.7, 0.9, 1.0, mech.purge_t * 0.9))
	# Readouts under the diagram
	Theme2.text(ci, Vector2(2, 196), "DRN>%s" % Rules.SECTION_SHORT[rt], Color8(240, 130, 60), 12)
	if mech.cameras_up() < Rules.CAMERAS:
		Theme2.text(ci, Vector2(86, 196), "CAM %d/4" % mech.cameras_up(), Theme2.AMBER, 12, 2)
	# Heat
	var hc := Theme2.RED if mech.heat >= 100.0 else (Color8(255, 120, 40) if mech.heat >= 80.0 else (Theme2.AMBER if mech.heat >= 50.0 else Theme2.GREEN))
	Theme2.label(ci, Vector2(2, 208), "HEAT", Theme2.RED, 8)
	Theme2.seg_bar(ci, Rect2(30, 200, 56, 9), mech.heat / Rules.HEAT_MAX, 13, hc)
	# critical mark on the bar
	var mx := 30.0 + 56.0 * Rules.HEAT_CRIT / Rules.HEAT_MAX
	ci.draw_rect(Rect2(mx, 199, 1, 11), Theme2.RED)
	var net: float = st.heat_in - st.heat_out
	Theme2.text(ci, Vector2(2, 224), "%d" % int(mech.heat), Theme2.RED if mech.heat >= 80.0 else Theme2.TEXT, 16)
	Theme2.text(ci, Vector2(86, 222), "%+.1f/s" % net, Theme2.AMBER if net > 0.0 else Theme2.GREEN, 12, 2)
	# Shield
	Theme2.label(ci, Vector2(2, 236), "SHLD", Theme2.SHIELD, 8)
	if st.shield_cap <= 0.0:
		Theme2.text(ci, Vector2(30, 236), "NOT WIRED", Theme2.DIM, 12)
	else:
		Theme2.seg_bar(ci, Rect2(30, 228, 56, 9), sfrac, 13, Theme2.RED if down else Theme2.SHIELD)
	# Core
	var draw_col := Theme2.RED if st.supply < 1.0 else (Theme2.AMBER if st.parasitic > 0.0 else Theme2.DIM)
	Theme2.label(ci, Vector2(2, 250), "CORE", Theme2.DIM, 8)
	Theme2.text(ci, Vector2(86, 250), "%d/%d" % [int(st.draw), int(st.available)], draw_col, 12, 2)
	if st.parasitic > 0.0:
		Theme2.text(ci, Vector2(86, 262), "-%d TICKS" % int(st.parasitic), Theme2.PURPLE, 12, 2)
	elif st.supply < 1.0:
		Theme2.text(ci, Vector2(86, 262), "BROWNOUT", Theme2.RED, 12, 2)
	else:
		Theme2.text(ci, Vector2(86, 262), "%d KILLS" % mech.kills, Theme2.DIM, 12, 2)


func hull_section_at(p: Vector2) -> int:
	var g := ((p - HULL_ORIGIN) / HULL_K).floor()
	var v := Hull.at(int(g.x), int(g.y))
	return v if v < Rules.SECTIONS else -1


func _hint_done() -> void:
	hint_stage += 1
	save_settings()


func _hints(ci: CanvasItem) -> void:
	if hint_stage >= 3 or sim.mission.number != 1 or not sim.mission.running():
		return
	var mech := sim.mech
	var msg := ""
	match hint_stage:
		0:
			if _hint_turned > 0.6:
				_hint_done()
				return
			msg = "SWIPE TO LOOK AROUND" if Theme2.touch else "DRAG TO LOOK AROUND"
		1:
			if mech.target >= 0:
				_hint_done()
				return
			var any := false
			for e in sim.enemies:
				if e.covered and not is_nan(feed.to_x(e.angle, mech.view)):
					any = true
			if not any:
				return
			msg = "TAP A CONTACT TO TARGET IT"
		2:
			if mech.kills > 0 or mech.rail_charge < 0.3:
				_hint_done()
				return
			if mech.rail_charge < 1.0 or mech.target < 0:
				return
			msg = "TAP IT AGAIN, OR RAIL, TO FIRE"
	if msg == "":
		return
	var y := FEED_POS.y + Feed.H - 34.0
	var w := Theme2.pix.get_string_size(msg, HORIZONTAL_ALIGNMENT_LEFT, -1, 8).x + 16.0
	var r := Rect2(FEED_POS.x + Feed.W / 2.0 - w / 2.0, y, w, 14)
	ci.draw_rect(r, Color(0, 0, 0, 0.75))
	ci.draw_rect(r, Color(Theme2.AMBER, 0.5 + 0.5 * Theme2.pulse(t, 1.2)), false, 1.0)
	Theme2.label(ci, Vector2(r.get_center().x, y + 10), msg, Theme2.AMBER, 8, 1)
	if hint_stage == 0:
		var k := 3.0 * sin(t * 4.0)
		ci.draw_colored_polygon(PackedVector2Array([Vector2(r.position.x - 10 + k, y + 7), Vector2(r.position.x - 4 + k, y + 3), Vector2(r.position.x - 4 + k, y + 11)]), Theme2.AMBER)
		ci.draw_colored_polygon(PackedVector2Array([Vector2(r.end.x + 10 - k, y + 7), Vector2(r.end.x + 4 - k, y + 3), Vector2(r.end.x + 4 - k, y + 11)]), Theme2.AMBER)


# ---------------------------------------------------------------------------------------
# Uplink and radio, under the feed
# ---------------------------------------------------------------------------------------

func _uplink(ci: CanvasItem) -> void:
	var u := sim.mech.uplink()
	var col := Theme2.CYAN if u < 100.0 else Theme2.GREEN
	Theme2.seg_bar(ci, UPLINK_BAR, u / 100.0, 42, col)
	Theme2.text(ci, Vector2(408, 216), "UPLINK %d%%" % int(u), col, 12, 2)


func _radio(ci: CanvasItem) -> void:
	var radio := sim.radio
	ci.draw_rect(RADIO, Color8(8, 12, 9))
	var face_r := Rect2(RADIO.position + Vector2(2, 2), Vector2(24, 24))
	var n := radio.log.size()
	if n > 0:
		var m: Dictionary = radio.log[n - 1]
		var v: int = m.voice
		var col := _voice_color(v)
		var sprite: Array = [Sprites.FACE_CONTROL, Sprites.FACE_CHIEF, Sprites.FACE_PILOT, Sprites.FACE_ENEMY, Sprites.FACE_SYSTEM][v]
		ci.draw_rect(face_r, Color(col, 0.15))
		Pixel.draw(ci, sprite, face_r.position)
		var talking: bool = radio.time - m.time < 3.0
		if talking and int(radio.time * 9.0) % 2 == 0:
			var mouth: Array = [[Vector2(10, 14), Vector2(3, 2), Color8(120, 40, 40)], [Vector2(10, 17), Vector2(4, 2), Color8(60, 20, 20)], [Vector2(10, 19), Vector2(4, 2), Color8(120, 40, 40)], [Vector2(5, 12), Vector2(14, 2), Color(Theme2.RED, 0.7)], []][v]
			if not mouth.is_empty():
				ci.draw_rect(Rect2(face_r.position + mouth[0], mouth[1]), mouth[2])
		Theme2.scanlines(ci, face_r, 0.2)
		if talking:
			for i in 6:
				var hgt := absf(sin(radio.time * 13.0 + i * 1.7)) * 7.0 + 1.0
				ci.draw_rect(Rect2(Vector2(face_r.position.x + 1 + i * 4, face_r.end.y + 21 - hgt), Vector2(3, hgt)), col)
	var y := RADIO.position.y + 13.0
	var first := maxi(n - 3, 0)
	for i in range(first, n):
		var m: Dictionary = radio.log[i]
		var col := _voice_color(m.voice)
		var body: String = m.text
		if i + 1 == n:
			var chars := int(maxf((radio.time - m.time) * 70.0, 1.0))
			body = body.substr(0, chars)
		Theme2.text(ci, Vector2(RADIO.position.x + 30, y), VOICE_NAME[m.voice], col, 12)
		Theme2.text(ci, Vector2(RADIO.position.x + 78, y), Theme2.fit(body, RADIO.end.x - RADIO.position.x - 80), Color.WHITE if i + 1 == n else Theme2.TEXT, 12)
		y += 14.0
	Theme2.scanlines(ci, RADIO, 0.08)


func _voice_color(v: int) -> Color:
	return [Theme2.CYAN, Theme2.AMBER, Theme2.GREEN, Theme2.RED, Color8(170, 180, 170)][v]


# ---------------------------------------------------------------------------------------
# Stations
# ---------------------------------------------------------------------------------------

## name, color, charge, status, status_col, enabled, lit
func _slots() -> Array:
	var mech := sim.mech
	var st: Dictionary = mech.st
	var c := sim.campaign
	var target := sim.target()
	var rail_dead: bool = mech.hp[Rules.Section.RAILGUN] <= 0.0
	var ac_dead: bool = mech.hp[Rules.Section.AUTOCANNON] <= 0.0
	var rail_ready: bool = mech.rail_charge >= 1.0 and not rail_dead
	var lock: bool = target != null and target.dist <= st.radar_range
	var radar_only: bool = target != null and not target.covered
	var hitp := 0.92 * (0.5 + 0.5 * mech.eff(Rules.Section.SENSORS))
	if target:
		if target.kind == Rules.Kind.SCOUT:
			hitp -= 0.15
		hitp *= target.aim_factor()
	var rail: Array
	if rail_dead:
		rail = ["DESTROYED", Theme2.RED]
	elif not c.matrix[Rules.Sys.RAIL]:
		rail = ["NOT WIRED", Theme2.DIM]
	elif not rail_ready:
		rail = ["%.1fs" % ((1.0 - mech.rail_charge) / st.rail_rate) if st.rail_rate > 0.0 else "NO POWER", Theme2.AMBER]
	elif not lock:
		rail = ["NO TARGET", Theme2.AMBER]
	elif radar_only:
		rail = ["%d%% RADAR" % int(hitp * 100.0), Theme2.AMBER]
	else:
		rail = ["%d%% HIT" % int(hitp * 100.0), Theme2.GREEN]
	var ac: Array
	if ac_dead:
		ac = ["DESTROYED", Theme2.RED]
	elif not c.matrix[Rules.Sys.AC]:
		ac = ["NOT WIRED", Theme2.DIM]
	elif mech.ac_firing:
		ac = ["%d/s" % int(st.ac_dps), Theme2.YELLOW]
	elif mech.weapons_free:
		ac = ["TRACKING", Theme2.GREEN]
	else:
		ac = ["SAFE", Theme2.DIM]
	var ecm_ok: bool = mech.ecm_cd <= 0.0 and st.eff[Rules.Sys.SENS] >= 5.0
	var nb := sim.boarders.size()
	var purge_ok: bool = mech.purge_cd <= 0.0 and mech.scram_t <= 0.0
	var purge: Array
	if mech.purge_cd > 0.0:
		purge = ["%ds" % int(ceilf(mech.purge_cd)), Theme2.DIM]
	elif nb > 0:
		purge = ["%d BOARDER%s" % [nb, "S" if nb > 1 else ""], Theme2.PURPLE]
	else:
		purge = ["+%d HEAT" % int(Rules.PURGE_HEAT), Theme2.DIM]
	return [
		["RAIL", Theme2.CYAN, mech.rail_charge, rail[0], rail[1], rail_ready and lock and mech.scram_t <= 0.0, rail_ready and lock],
		["A/C", Theme2.YELLOW, 1.0 - fmod(t * 1.5, 1.0) if mech.ac_firing else 1.0, ac[0], ac[1], not ac_dead and c.matrix[Rules.Sys.AC], mech.weapons_free and not ac_dead],
		["ECM", Theme2.CYAN, 1.0 - mech.ecm_cd / 28.0, "READY" if ecm_ok else ("%ds" % int(mech.ecm_cd) if mech.ecm_cd > 0.0 else "NO SENSORS"), Theme2.GREEN if ecm_ok else Theme2.DIM, ecm_ok, ecm_ok],
		["VENT", Color8(150, 230, 255), 1.0 - mech.vent_cd / 25.0, "VENTING" if mech.vent_t > 0.0 else ("%ds" % int(mech.vent_cd) if mech.vent_cd > 0.0 else "-20 HEAT/s"), Theme2.GREEN if mech.vent_cd <= 0.0 else Theme2.DIM, mech.vent_cd <= 0.0, mech.vent_t > 0.0],
		["SCRAM", Theme2.AMBER, 1.0 - mech.scram_cd / 30.0, "DEAD %ds" % int(mech.scram_t) if mech.scram_t > 0.0 else ("%ds" % int(mech.scram_cd) if mech.scram_cd > 0.0 else "KILL BUSES"), Theme2.GREEN if mech.scram_cd <= 0.0 else Theme2.DIM, mech.scram_cd <= 0.0, mech.scram_t > 0.0],
		["PURGE", Theme2.PURPLE, 1.0 - mech.purge_cd / Rules.PURGE_CD, purge[0], purge[1], purge_ok, purge_ok and nb > 0],
	]


func _stations(ci: CanvasItem) -> void:
	var slots := _slots()
	for i in 6:
		var s: Array = slots[i]
		var r := _station_rects[i]
		var color: Color = s[1]
		var enabled: bool = s[5]
		var lit: bool = s[6]
		var pressed := _down_station == i
		ci.draw_rect(r, Theme2.dim(color, 0.22) if lit else Theme2.PANEL_DARK)
		ci.draw_rect(r, color if lit else (Theme2.dim(color, 0.7) if pressed else Theme2.EDGE_HI), false, 1.0)
		var tint := Color.WHITE if enabled or lit else Color8(110, 110, 120)
		var flash := 0.6 if i == 0 and sim.mech.shake > 0.5 else 0.0
		Pixel.draw(ci, Sprites.HOTBAR[i], r.position + Vector2(2, 5), 1.0, false, tint, flash)
		Theme2.label(ci, r.position + Vector2(29, 14), s[0], color if enabled or lit else Theme2.DIM, 8)
		Theme2.seg_bar(ci, Rect2(r.position + Vector2(29, 19), Vector2(34, 5)), s[2], 8, color)
		Theme2.text(ci, r.position + Vector2(69, 37), s[3], s[4], 12, 2)
		if not Theme2.touch:
			Theme2.label(ci, r.position + Vector2(69, 14), str(i + 1), Theme2.dim(color, 0.6), 8, 2)


func _station(i: int) -> void:
	var mech := sim.mech
	match i:
		0: sim.cmd_fire_rail = true
		1: mech.weapons_free = not mech.weapons_free
		2: sim.cmd_ecm = true
		3: sim.cmd_vent = true
		4: sim.cmd_scram = true
		5: sim.cmd_purge = true


# ---------------------------------------------------------------------------------------
# Modals
# ---------------------------------------------------------------------------------------

func _modal_frame(ci: CanvasItem, size: Vector2, border: Color) -> Rect2:
	ci.draw_rect(Rect2(0, 0, 480, 270), Color(0, 0, 0, 0.67))
	var r := Rect2(Vector2(240, 135) - size / 2.0, size)
	ci.draw_rect(r.grow(2), Theme2.EDGE_LO)
	ci.draw_rect(r, Theme2.PANEL)
	ci.draw_rect(r, border, false, 1.0)
	return r


func _modals(ci: CanvasItem) -> void:
	var mission := sim.mission
	var mech := sim.mech
	_modal_btn = Rect2()
	_gyro_btn = Rect2()
	_modal_btn2 = Rect2()
	if mission.briefing:
		var r := _modal_frame(ci, Vector2(440, 200), Theme2.AMBER)
		Theme2.label(ci, r.position + Vector2(10, 16), "OPERATION %02d // BRIEFING" % mission.number, Theme2.AMBER, 8)
		var face := Rect2(r.end.x - 40, r.position.y + 8, 32, 32)
		ci.draw_rect(face, Color(Theme2.CYAN, 0.15))
		Pixel.draw(ci, Sprites.FACE_CONTROL, face.position + Vector2(4, 4))
		Theme2.scanlines(ci, face, 0.2)
		var lines := [
			["CONTROL", Theme2.CYAN, "Op %02d. The uplink's dark again. Same job: hold until the drones bring it up." % mission.number],
			["ODA", Theme2.AMBER, "She's carrying scars now. Watch what lights up on the hull."],
		] if mission.number > 1 else [
			["CONTROL", Theme2.CYAN, "The uplink on your mast is down. Without it the shield grid stays dark tonight."],
			["ODA", Theme2.AMBER, "Hold position; the drones rebuild the uplink. Your LEFT camera is dead: anything west is radar-only, wider error, worse shots."],
			["ODA", Theme2.AMBER, "Every shot heats the core. Past 100, containment goes in 12 seconds. VENT dumps heat but drops shields; SCRAM kills everything for 6."],
			["CONTROL", Theme2.CYAN, "Scouts, Crabs beyond your radar, Carriers that board you: PURGE fries boarders, or send the drones at them. What's lit on the hull is what's being hit."],
		]
		var y := r.position.y + 34.0
		for l in lines:
			Theme2.text(ci, Vector2(r.position.x + 10, y), l[0], l[1], 12)
			var h := Theme2.text_wrap(ci, Vector2(r.position.x + 64, y), l[2], r.size.x - 116, Theme2.TEXT, 12)
			y += maxf(h, 12.0) + 3.0
		Theme2.text(ci, Vector2(r.position.x + 10, r.end.y - 28), "drag or turn the phone to look. tap a blip to target.", Theme2.DIM, 12)
		_modal_btn = Rect2(r.get_center().x - 60, r.end.y - 22, 120, 16)
		Theme2.button(ci, _modal_btn, "BEGIN OPERATION", Theme2.AMBER, true, true)
		return
	if mission.outcome != Rules.Outcome.NONE:
		var victory := mission.outcome == Rules.Outcome.VICTORY
		var info: Array = {
			Rules.Outcome.VICTORY: ["UPLINK RESTORED", "The shield grid flickers on across the city. Bastion limps home.", Theme2.CYAN],
			Rules.Outcome.MELTDOWN: ["REACTOR OVERLOAD", "Containment failed. WF-7 BASTION is lost with her pilot.", Theme2.RED],
			Rules.Outcome.EJECTED: ["PILOT EJECTED", "The core went critical seconds after you cleared the hatch.", Theme2.RED],
		}[mission.outcome]
		var col: Color = info[2]
		var r := _modal_frame(ci, Vector2(360, 190), col)
		if not victory:
			ci.draw_rect(Rect2(0, 0, 480, 3), Color(Theme2.RED, sin(t * 2.0) * 0.5 + 0.5))
		Theme2.label(ci, Vector2(r.get_center().x, r.position.y + 18), info[0], col, 16, 1)
		Theme2.text(ci, Vector2(r.get_center().x, r.position.y + 34), info[1], Theme2.TEXT, 12, 1)
		# Bastion's silhouette, lit by what happened to her
		var vo := r.position + Vector2(14, 48)
		ci.draw_rect(Rect2(vo - Vector2(4, 4), Vector2(88, 100)), Color8(8, 10, 14))
		for y in Hull.H:
			for x in Hull.W:
				var v := Hull.at(x, y)
				if v == Hull.EMPTY:
					continue
				var base := Color8(70, 92, 84) if victory else Color8(60, 40, 40)
				var d := Hull.detail_at(x, y)
				var c := Hull.shade(base, d, t) if d != Hull.D.NONE else base
				if not victory and v == Rules.Section.REACTOR:
					c = Color8(255, int(120 + 100 * (sin(t * 5.0) * 0.5 + 0.5)), 40)
				if victory and v == Rules.Section.UPLINK and fmod(t * 2.0, 1.0) < 0.5:
					c = Theme2.CYAN
				ci.draw_rect(Rect2(vo + Vector2(x, y) * 2.0, Vector2(2, 2)), c)
		if not victory:
			feed._boom(ci, vo + Vector2(40, 36), 2.0, fmod(t * 0.7, 1.0), 5)
		var stats := [["MISSION TIME", Theme2.fmt_clock(mission.time)], ["DESTROYED", str(mech.kills)], ["PEAK HEAT", str(int(mech.heat_peak))], ["UPLINK", "%d%%" % int(mech.uplink())]]
		var y := r.position.y + 62.0
		for s in stats:
			Theme2.text(ci, Vector2(r.position.x + 120, y), s[0], Theme2.DIM, 12)
			Theme2.text(ci, Vector2(r.end.x - 16, y), s[1], Theme2.TEXT, 12, 2)
			y += 14.0
		if victory:
			_modal_btn = Rect2(r.position.x + 120, r.end.y - 26, r.size.x - 136, 16)
			Theme2.button(ci, _modal_btn, "RETURN TO HANGAR", col, true, true)
		else:
			_modal_btn = Rect2(r.position.x + 120, r.end.y - 26, 108, 16)
			Theme2.button(ci, _modal_btn, "RETRY OP %02d" % mission.number, Theme2.AMBER, true, true)
			_modal_btn2 = Rect2(r.end.x - 124, r.end.y - 26, 108, 16)
			Theme2.button(ci, _modal_btn2, "NEW CAMPAIGN", col, true, false)
		return
	if mission.paused:
		var r := _modal_frame(ci, Vector2(300, 170), Theme2.DIM)
		Theme2.label(ci, Vector2(r.get_center().x, r.position.y + 18), "PAUSED", Theme2.AMBER, 12, 1)
		var help: Array
		if Theme2.touch:
			help = [
				["swipe", "look around; flick for the next camera"],
				["tap blip", "select target; tap it again to fire"],
				["tap hull", "send repair drones there"],
				["stations", "rail, a/c, ECM, vent, scram, purge"],
			]
		else:
			help = [
				["drag / gyro", "look around (TAB snaps cameras)"],
				["tap blip", "select target (T cycles)"],
				["tap hull", "send repair drones there"],
				["1-6", "rail, a/c, ECM, vent, scram, purge"],
				["SPACE", "fire railgun"],
			]
		var y := r.position.y + 38.0
		for h in help:
			Theme2.text(ci, Vector2(r.position.x + 70, y), h[0], Theme2.AMBER, 12, 2)
			Theme2.text(ci, Vector2(r.position.x + 78, y), h[1], Theme2.TEXT, 12)
			y += 14.0
		_gyro_btn = Rect2(r.position.x + 16, r.end.y - 26, 96, 18)
		Theme2.button(ci, _gyro_btn, "GYRO %s" % ("ON" if gyro_enabled else "OFF"), Theme2.CYAN, true, gyro_enabled)
		_modal_btn = Rect2(r.end.x - 112, r.end.y - 26, 96, 18)
		Theme2.button(ci, _modal_btn, "RESUME", Theme2.AMBER, true, true)


func _modal_tap(p: Vector2) -> void:
	var mission := sim.mission
	if mission.paused and _gyro_btn.has_point(p):
		gyro_enabled = not gyro_enabled
		save_settings()
		return
	if mission.outcome != Rules.Outcome.NONE and _modal_btn2.has_point(p):
		request = "new_campaign"
		return
	if not _modal_btn.has_point(p):
		return
	if mission.briefing:
		mission.briefing = false
	elif mission.outcome != Rules.Outcome.NONE:
		request = "hangar" if mission.outcome == Rules.Outcome.VICTORY else "retry"
	elif mission.paused:
		mission.paused = false


# ---------------------------------------------------------------------------------------
# Input
# ---------------------------------------------------------------------------------------

func press(p: Vector2) -> void:
	_press = p
	_dragged = 0.0
	_down_station = -1
	_press_in_feed = false
	if modal_open():
		return
	for i in 6:
		if _station_rects[i].has_point(p):
			_down_station = i
			return
	_press_in_feed = Rect2(FEED_POS, Vector2(Feed.W, Feed.H)).has_point(p)
	_drag_dx = 0.0
	_press_cam = Rules.camera_for(sim.mech.view)


func drag(p: Vector2, rel: Vector2) -> void:
	_dragged += rel.length()
	if _press_in_feed and sim.mission.running():
		_drag_dx += rel.x
		_hint_turned += absf(rel.x) / Feed.W * Rules.CAMERA_FOV
		_snap = -1.0
		_still_t = 0.0
		sim.mech.view = fposmod(sim.mech.view - rel.x / Feed.W * Rules.CAMERA_FOV, TAU)


func release(p: Vector2) -> void:
	var station := _down_station
	_down_station = -1
	if _press_in_feed:
		_press_in_feed = false
		if absf(_drag_dx) > 24.0:
			# A flick moves to the next camera in that direction even if the drag was short.
			var cam := Rules.camera_for(sim.mech.view)
			if cam == _press_cam:
				cam = (cam + (1 if _drag_dx < 0.0 else -1) + Rules.CAMERAS) % Rules.CAMERAS
			_snap = Rules.camera_bearing(cam)
			return
	if modal_open():
		_modal_tap(p)
		return
	if station >= 0:
		if _station_rects[station].has_point(p) and _slots()[station][5]:
			_station(station)
		return
	if PAUSE_BTN.has_point(p):
		sim.mission.paused = true
		return
	if _dragged > 5.0:
		return
	var mech := sim.mech
	if RADAR.has_point(p):
		_pick_blip(p)
	elif HULL_BOX.has_point(p):
		var s := hull_section_at(p)
		if s >= 0:
			mech.repair_target = s
	elif feed.eject_btn.has_point(p):
		sim.cmd_eject = true
	elif Rect2(FEED_POS, Vector2(Feed.W, Feed.H)).has_point(p):
		_pick_in_feed(p)


func _pick_blip(p: Vector2) -> void:
	var mech := sim.mech
	var c := Vector2(44, 44)
	var k := 35.0 / Rules.RADAR_SCALE_M
	var best := -1
	var best_d := 14.0
	for e in sim.enemies:
		if e.dist > mech.st.radar_range:
			continue
		var pos := Theme2.polar(c, e.est_angle, minf(e.est_dist, Rules.RADAR_SCALE_M) * k)
		var d := pos.distance_to(p)
		if d < best_d:
			best_d = d
			best = e.id
	if best >= 0:
		mech.target = best
	else:
		# A tap on empty scope cycles through what the radar can see.
		cycle_target()


func _pick_in_feed(p: Vector2) -> void:
	var mech := sim.mech
	var best := -1
	var best_d := 24.0
	for e in sim.enemies:
		if e.dist > mech.st.radar_range or not e.covered:
			continue
		var x := feed.to_x(e.angle, mech.view)
		if is_nan(x):
			continue
		var d := absf(FEED_POS.x + x - p.x)
		if d < best_d:
			best_d = d
			best = e.id
	if best < 0:
		return
	if best == mech.target and _slots()[0][5]:
		_station(0)  # a second tap on the locked contact fires the rail
	else:
		mech.target = best


func cycle_target() -> void:
	var mech := sim.mech
	var vis: Array = []
	for e in sim.enemies:
		if e.dist <= mech.st.radar_range:
			vis.append(e)
	if vis.is_empty():
		return
	vis.sort_custom(func(a, b): return a.dist < b.dist)
	var i := 0
	for j in vis.size():
		if vis[j].id == mech.target:
			i = j + 1
	mech.target = vis[i % vis.size()].id


func turn_camera(step: int) -> void:
	var mech := sim.mech
	var cam := (Rules.camera_for(mech.view) + step + Rules.CAMERAS) % Rules.CAMERAS
	_snap = Rules.camera_bearing(cam)
	_still_t = 0.0
	_hint_turned += 1.0


func key(keycode: int, shift: bool) -> void:
	var mission := sim.mission
	if keycode == KEY_P or keycode == KEY_ESCAPE:
		if mission.outcome == Rules.Outcome.NONE and not mission.briefing:
			mission.paused = not mission.paused
		return
	if modal_open():
		if keycode == KEY_ENTER or keycode == KEY_SPACE:
			_modal_tap(_modal_btn.get_center())
		return
	match keycode:
		KEY_SPACE: sim.cmd_fire_rail = true
		KEY_V: sim.cmd_vent = true
		KEY_X: sim.cmd_scram = true
		KEY_E: sim.cmd_ecm = true
		KEY_T: cycle_target()
		KEY_TAB: turn_camera(-1 if shift else 1)
		KEY_LEFT: turn_camera(-1)
		KEY_RIGHT: turn_camera(1)
		KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6:
			var i := keycode - KEY_1
			if _slots()[i][5]:
				_station(i)
		KEY_BACKSPACE:
			if sim.mech.overload >= 0.0:
				sim.cmd_eject = true
