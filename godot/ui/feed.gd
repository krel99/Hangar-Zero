## The camera feed: a 320x208 window onto the city at the bearing the pilot is looking.
## Arcs with no working hull camera show static.
class_name Feed
extends RefCounted

const W := 320.0
const H := 208.0
const FOV := Rules.CAMERA_FOV

var sim: Sim
var origin := Vector2.ZERO
var t := 0.0
## Hit rect of the EJECT button while the reactor is overloading (empty otherwise).
var eject_btn := Rect2()


func set_sim(s: Sim) -> void:
	sim = s


func px(x: float, y: float) -> Vector2:
	return origin + Vector2(x, y)


## Screen x of a world bearing, or NAN if outside the view (with a little margin).
func to_x(angle: float, view: float) -> float:
	var rel := Rules.angle_diff(angle, view)
	if absf(rel) >= FOV * 0.6:
		return NAN
	return W / 2.0 + rel / (FOV / 2.0) * (W / 2.0)


func ground_y(dist: float) -> float:
	return City.HORIZON + pow(1.0 - clampf(dist / 1100.0, 0.0, 1.0), 1.6) * 72.0


func _pano(ci: CanvasItem, tex: Texture2D, u0: float) -> void:
	# u0 in [0,1): left edge of the view in panorama space; the view spans FOV/TAU.
	var pw := float(City.PANO_W)
	var x0 := fposmod(u0, 1.0) * pw
	var span := FOV / TAU * pw
	var first := minf(span, pw - x0)
	var scale := W / span
	ci.draw_texture_rect_region(tex, Rect2(origin, Vector2(first * scale, H)), Rect2(x0, 0, first, H))
	if first < span:
		ci.draw_texture_rect_region(tex, Rect2(origin + Vector2(first * scale, 0), Vector2((span - first) * scale, H)), Rect2(0, 0, span - first, H))


func draw(ci: CanvasItem, now: float) -> void:
	t = now
	var mech := sim.mech
	var view := mech.view
	City.ensure()
	_pano(ci, City.far_tex, (view * 0.6 - FOV / 2.0) / TAU)
	_pano(ci, City.near_tex, (view - FOV / 2.0) / TAU)
	_sky_life(ci, view)
	_fog(ci, view)
	_enemies(ci, view)
	_shells(ci, view)
	_effects(ci, view)
	_arms(ci, view)
	_dead_arcs(ci, view)
	_hud(ci, view)
	Theme2.scanlines(ci, Rect2(origin, Vector2(W, H)), 0.09)


func _sky_life(ci: CanvasItem, view: float) -> void:
	# Ash and embers
	for i in 30:
		var h := Rng.hash2(i, 0, 23)
		var speed := 4.0 + (h % 7)
		var y := fmod(((h >> 8) & 0xFFF) * 0.37 + t * speed, H)
		var x := fposmod(((h >> 16) & 0xFFF) * 0.53 + sin(t * 0.7 + i) * 6.0, W)
		ci.draw_rect(Rect2(px(floorf(x), floorf(y)), Vector2.ONE), Color8(255, 150, 60) if h % 9 == 0 else Color8(110, 96, 100))
	# Searchlights and tower blinkers
	for i in 3:
		var brg: float = [0.7, 3.6, 5.2][i]
		var x := to_x(brg, view)
		if is_nan(x):
			continue
		var sweep := sin(t * 0.25 + [0.0, 2.1, 4.4][i]) * 0.9
		var base := px(x, City.HORIZON - 2)
		var tip := base + Vector2(sweep * 90.0, -110.0)
		var a := 0.07 + 0.04 * sin(t * 0.9 + i)
		ci.draw_colored_polygon(PackedVector2Array([base + Vector2(-2, 0), base + Vector2(2, 0), tip + Vector2(18, 0), tip - Vector2(18, 0)]), Color(0.9, 0.86, 0.75, a))
	for i in 6:
		var brg: float = [0.3, 1.9, 2.8, 4.1, 4.9, 5.9][i]
		var hgt: float = [54.0, 40.0, 62.0, 36.0, 48.0, 58.0][i]
		var x := to_x(brg, view)
		if not is_nan(x) and fmod(t * 0.8 + i * 0.37, 1.0) < 0.12:
			ci.draw_rect(Rect2(px(floorf(x), City.HORIZON - hgt), Vector2(2, 2)), Color8(255, 70, 50))


func _fog(ci: CanvasItem, view: float) -> void:
	for i in 50:
		var h := Rng.hash2(i, 5, 41)
		var w := 6.0 + (h % 20)
		var x := fposmod(((h >> 8) % 320) + t * (2.0 + (h % 3)) - rad_to_deg(view) * 3.55, W + 40.0) - 20.0
		var y := City.HORIZON + 4.0 + ((h >> 16) % 30)
		var a := (14 + ((h >> 20) % 18)) / 255.0
		ci.draw_rect(Rect2(px(x, y), Vector2(w, 2)), Color(0.59, 0.47, 0.51, a))


func _enemies(ci: CanvasItem, view: float) -> void:
	var mech := sim.mech
	var list := sim.enemies.duplicate()
	list.sort_custom(func(a, b): return a.dist > b.dist)
	for e in list:
		if not mech.covers(e.angle):
			continue
		var x := to_x(e.angle, view)
		if is_nan(x):
			continue
		var moving: bool = e.state == Rules.EState.APPROACH or e.kind == Rules.Kind.SCOUT
		var walk := int(t * 6.0 + e.serial) % 2 == 1
		var rows: Array
		match e.kind:
			Rules.Kind.SCOUT: rows = Sprites.SCOUT_WALK if moving and walk else Sprites.SCOUT
			Rules.Kind.ARTILLERY: rows = Sprites.ARTILLERY
			_: rows = Sprites.CARRIER
		var k := 3.0 if e.dist < 230.0 else (2.0 if e.dist < 520.0 else (1.5 if e.dist < 800.0 else 1.0))
		var gy := ground_y(e.dist)
		var bob := 0.0
		if e.kind == Rules.Kind.CARRIER:
			bob = roundf(sin(t * 1.6 + e.serial) * 2.0)
		elif moving:
			bob = roundf(sin(t * 8.0 + e.serial) * 0.6)
		var recoil := roundf((e.reveal - 2.6) * 8.0) if (e.kind == Rules.Kind.ARTILLERY and e.reveal > 2.6) else 0.0
		var facing_left: bool = e.orbit < 0.0 and e.kind == Rules.Kind.SCOUT
		var sw := Pixel.grid_w(rows) * k
		var sh := rows.size() * k
		var pos := px(roundf(x - sw / 2.0 + (-recoil if facing_left else recoil)), roundf(gy - sh + bob))
		ci.draw_rect(Rect2(px(x - sw * 0.45, gy - 1), Vector2(sw * 0.9, 3.0 * maxf(k, 1.0) * 0.6)), Color(0, 0, 0, 0.35 if e.kind != Rules.Kind.CARRIER else 0.24))
		var fog := clampf((e.dist - 300.0) / 700.0, 0.0, 0.7)
		var tint := Color(1.0 - fog * (1.0 - 150.0 / 255.0), 1.0 - fog * (1.0 - 120.0 / 255.0), 1.0 - fog * (1.0 - 135.0 / 255.0))
		Pixel.draw(ci, rows, pos, k, facing_left, tint, minf(e.hit_flash * 2.0, 0.9))
		if e.id == mech.target:
			var r := Rect2(pos, Vector2(sw, sh)).grow(3)
			var c := Color8(255, 80, 60)
			for corner in [[r.position, Vector2(1, 1)], [Vector2(r.end.x, r.position.y), Vector2(-1, 1)], [Vector2(r.position.x, r.end.y), Vector2(1, -1)], [r.end, Vector2(-1, -1)]]:
				var p: Vector2 = corner[0]
				var d: Vector2 = corner[1]
				ci.draw_line(p, p + Vector2(6 * d.x, 0), c)
				ci.draw_line(p, p + Vector2(0, 6 * d.y), c)
			Theme2.text(ci, Vector2(r.get_center().x, r.position.y - 3), "%s %dm" % [e.callsign(), int(e.dist)], c, 12, 1)
	for d in sim.drones:
		if not mech.covers(d.angle):
			continue
		var x := to_x(d.angle, view)
		if not is_nan(x):
			var y := ground_y(d.dist) - 18.0 - sin(d.dist * 0.05) * 4.0
			Pixel.draw(ci, Sprites.DRONE, px(x, y), 2.0 if d.dist < 200.0 else 1.0)


func _shells(ci: CanvasItem, view: float) -> void:
	for s in sim.shells:
		var x := to_x(s.angle, view)
		if is_nan(x):
			continue
		var f := s.t / s.flight
		var y := 10.0 + f * (H - 30.0)
		var xx := x + (W / 2.0 - x) * f
		ci.draw_rect(Rect2(px(xx, y), Vector2(2, 2)), Theme2.YELLOW)
		ci.draw_rect(Rect2(px(xx, y - 3), Vector2(2, 3)), Color(1, 0.59, 0.24, 0.47))


func _arm_slots(view: float) -> Dictionary:
	# Where the arms sit for this view: rail on the right when looking forward, etc.
	var rel := Rules.angle_diff(0.0, view)  # where "forward" is relative to the view
	var fwd_x := W / 2.0 + rel / (FOV / 2.0) * (W / 2.0)
	return {"rail": fwd_x + 100.0, "ac": fwd_x - 100.0}


func _arms(ci: CanvasItem, view: float) -> void:
	var mech := sim.mech
	var slots := _arm_slots(view)
	var ah := Sprites.ARM_RAIL.size()
	var rail_x: float = slots.rail
	var ac_x: float = slots.ac
	var rail_dead := mech.hp[Rules.Section.RAILGUN] <= 0.0
	var ac_dead := mech.hp[Rules.Section.AUTOCANNON] <= 0.0
	var dead_tint := Color8(90, 70, 70)
	if rail_x > -40.0 and rail_x < W + 40.0:
		var recoil := roundf(mech.shake * 3.0) if mech.shake > 0.3 else 0.0
		var accent: Color = Campaign.MAKER_ACCENT[sim.campaign.makers[Rules.Section.RAILGUN]]
		Pixel.draw(ci, Sprites.ARM_RAIL, px(rail_x - 10.0 + recoil, H - ah + 4.0 + recoil), 1.0, false, dead_tint if rail_dead else Color.WHITE, 0.0, Theme2.DIM if rail_dead else accent)
	if ac_x > -40.0 and ac_x < W + 40.0:
		var jitter := float(int(t * 60.0) % 2) if mech.ac_firing else 0.0
		var accent: Color = Campaign.MAKER_ACCENT[sim.campaign.makers[Rules.Section.AUTOCANNON]]
		Pixel.draw(ci, Sprites.ARM_AC, px(ac_x - 38.0, H - ah + 4.0 + jitter), 1.0, true, dead_tint if ac_dead else Color.WHITE, 0.0, Theme2.DIM if ac_dead else accent)


func _boom(ci: CanvasItem, c: Vector2, s: float, life: float, seed_v: int) -> void:
	if life > 0.25:
		var sl := (life - 0.25) / 0.75
		for i in 8:
			var h := Rng.hash2(i, seed_v, 21)
			var dx := ((h % 100) / 50.0 - 1.0) * (3.0 + sl * 8.0) * s
			var dy := -(((h >> 8) % 100) / 100.0 + sl * 2.2) * 6.0 * s
			var g := (70 + ((h >> 16) % 40)) / 255.0
			ci.draw_rect(Rect2(c + Vector2(dx, dy), Vector2.ONE * s * (1.5 + sl * 2.0)), Color(g, g - 0.04, g - 0.055, (1.0 - sl) * 0.67))
	for i in 6:
		var h := Rng.hash2(i, seed_v, 33)
		var vx := ((h % 100) / 50.0 - 1.0) * 14.0
		var vy := -(6.0 + ((h >> 8) % 100) / 100.0 * 12.0)
		var tt := life * 1.6
		var pt := c + Vector2(vx * tt, vy * tt + 24.0 * tt * tt) * s
		var col := Color8(255, 170, 60) if (h >> 16) % 3 == 0 else Color8(70, 62, 60)
		ci.draw_rect(Rect2(pt, Vector2.ONE * s * 1.2), Theme2.dim(col, 1.0 - life * 0.6))
	var r := (2.0 + life * 6.0) * s
	var cols := [Color.WHITE, Theme2.YELLOW, Color8(255, 140, 50), Color8(180, 60, 40), Color8(60, 50, 50)]
	var stage := mini(int(life * 4.0), 4)
	for i in 12:
		var h := Rng.hash2(i, seed_v, 9)
		var a := (h % 628) / 100.0
		var d := ((h >> 10) % 100) / 100.0 * r
		var pt := c + Vector2(cos(a) * d, sin(a) * d * 0.7)
		ci.draw_rect(Rect2(pt, Vector2.ONE * maxf(s * (2.0 - life), s * 0.6)), cols[mini(stage + (i % 2), 4)])


func _effects(ci: CanvasItem, view: float) -> void:
	var slots := _arm_slots(view)
	var rail_origin := px(slots.rail + 14.0, H)
	var ac_origin := px(slots.ac - 14.0, H)
	for f in sim.fx:
		var life: float = f.t / f.ttl
		var x := to_x(f.angle, view)
		var target := px(x, ground_y(f.dist) - 10.0) if not is_nan(x) else Vector2.INF
		match f.kind:
			Rules.Fx.RAIL:
				if target != Vector2.INF:
					var a := 1.0 - life
					ci.draw_line(rail_origin, target, Color(Theme2.CYAN, a), 6.0 * a)
					ci.draw_line(rail_origin, target, Color(1, 1, 1, a), 2.0 * a)
				if life < 0.3:
					var fl := 1.0 - life / 0.3
					var r := 6.0 + 10.0 * fl
					ci.draw_colored_polygon(PackedVector2Array([rail_origin + Vector2(-r, 0), rail_origin + Vector2(0, -r * 1.6), rail_origin + Vector2(r, 0), rail_origin + Vector2(0, r * 0.4)]), Color(0.78, 0.94, 1.0, 0.7 * fl))
			Rules.Fx.TRACER:
				if target != Vector2.INF:
					var a := ac_origin + (target - ac_origin) * (0.2 + life * 0.8)
					var b := ac_origin + (target - ac_origin) * (life * 0.8)
					ci.draw_line(b, a, Theme2.YELLOW, 1.5)
					ci.draw_rect(Rect2(ac_origin - Vector2.ONE * 3.0 * (1.0 - life), Vector2.ONE * 6.0 * (1.0 - life)), Color8(255, 230, 140))
			Rules.Fx.EXPLOSION, Rules.Fx.MUZZLE:
				if target != Vector2.INF:
					var big := 1.0 if f.kind == Rules.Fx.EXPLOSION else 0.4
					_boom(ci, target, big * (3.0 if f.dist < 300.0 else 1.6), life, int(f.angle * 1000.0))
			Rules.Fx.SHELL_IMPACT:
				if not is_nan(x):
					var base := px(x, H - 12.0)
					_boom(ci, base, 4.0, life, 77)
					for i in 14:
						var h := Rng.hash2(i, 77, 13)
						var rise := life * (30.0 + ((h >> 8) % 40))
						var dx := ((h % 100) / 50.0 - 1.0) * (4.0 + rise * 0.5)
						var g := (90 + ((h >> 16) % 50)) / 255.0
						ci.draw_rect(Rect2(base + Vector2(dx, -rise), Vector2.ONE * (2.0 + life * 3.0)), Color(g, g - 0.08, g - 0.14, (1.0 - life) * 0.6))
			Rules.Fx.ECM:
				ci.draw_arc(px(W / 2.0, H), life * W, 0.0, TAU, 48, Color(Theme2.CYAN, 1.0 - life), 3.0)


## Static over the columns whose hull camera is dead.
func _dead_arcs(ci: CanvasItem, view: float) -> void:
	var mech := sim.mech
	var frame := int(t * 20.0)
	for cam in Rules.CAMERAS:
		if mech.camera_ok(cam):
			continue
		var c := Rules.camera_bearing(cam)
		var lo := Rules.angle_diff(c - FOV / 2.0, view)
		var hi := Rules.angle_diff(c + FOV / 2.0, view)
		if hi < lo:
			hi += TAU
		var x0 := clampf(W / 2.0 + lo / (FOV / 2.0) * (W / 2.0), 0.0, W)
		var x1 := clampf(W / 2.0 + hi / (FOV / 2.0) * (W / 2.0), 0.0, W)
		if x1 - x0 < 1.0:
			continue
		var r := Rect2(px(x0, 0), Vector2(x1 - x0, H))
		ci.draw_rect(r, Color8(8, 8, 10))
		var n := int((x1 - x0) * 4.0)
		for i in n:
			var h := Rng.hash2(i, frame, 17 + cam)
			var x := x0 + (h % int(x1 - x0 + 1))
			var y := (h >> 9) % int(H)
			var v := (10 + (h >> 20) % 70) / 255.0
			var col := Color(v, v, v)
			match (h >> 27) % 9:
				0: col = Color(v + 0.12, v, v)
				1: col = Color(v, v, v + 0.16)
			ci.draw_rect(Rect2(px(x, y), Vector2(2, 1)), col)
		for b in 3:
			var band := (frame / 2 + b * 70) % int(H)
			ci.draw_rect(Rect2(px(x0, band), Vector2(x1 - x0, 10)), Color(1, 1, 1, (10 + b * 4) / 255.0))
		# The join with the live camera is a bezel edge, not a hard cut.
		for edge in [x0, x1]:
			if edge <= 0.5 or edge >= W - 0.5:
				continue
			var live_dir := 1.0 if edge == x1 else -1.0
			for i in 8:
				var a := (1.0 - i / 8.0) * 0.55
				ci.draw_rect(Rect2(px(edge + live_dir * i - (0.0 if live_dir > 0 else 1.0), 0), Vector2(1, H)), Color(0, 0, 0, a))
			ci.draw_rect(Rect2(px(edge - 1, 0), Vector2(2, H)), Color8(30, 32, 38))
			ci.draw_rect(Rect2(px(edge - (1.0 if live_dir > 0 else 0.0), 0), Vector2(1, H)), Color8(70, 76, 86))
		if x1 - x0 > 60.0 and fmod(t * 1.5, 1.0) < 0.7:
			Theme2.label(ci, px((x0 + x1) / 2.0, H / 2.0), "NO SIGNAL", Theme2.RED, 8, 1)
			Theme2.text(ci, px((x0 + x1) / 2.0, H / 2.0 + 12), "CAM %s DEAD" % Rules.CAMERA_NAMES[cam], Theme2.DIM, 12, 1)


func _hud(ci: CanvasItem, view: float) -> void:
	var mech := sim.mech
	# Bearing tape
	for i in range(-9, 10):
		var deg := roundf(rad_to_deg(view) / 5.0) * 5.0 + i * 5.0
		var rel := Rules.angle_diff(deg_to_rad(deg), view)
		var x := W / 2.0 + rel / (FOV / 2.0) * (W / 2.0)
		var d := int(fposmod(deg, 360.0))
		var tall := d % 15 == 0
		ci.draw_rect(Rect2(px(x, 3), Vector2(1, 4 if tall else 2)), Theme2.dim(Theme2.GREEN, 0.8))
		if d % 45 == 0:
			var lbl: String = {0: "N", 90: "E", 180: "S", 270: "W"}.get(d, "%03d" % d)
			Theme2.text(ci, px(x, 15), lbl, Theme2.GREEN, 12, 1)
	# Target readout: who, how far, the odds
	var tgt := sim.target()
	if tgt and tgt.dist <= mech.st.radar_range:
		var odds := sim.hit_chance(tgt)
		var col := Theme2.GREEN if tgt.covered else Theme2.AMBER
		Theme2.text(ci, px(6, 30), "%s  %dm" % [tgt.callsign(), int(tgt.est_dist)], col, 12)
		Theme2.text(ci, px(6, 42), "%d%% %s" % [int(odds * 100.0), "HIT" if tgt.covered else "RADAR"], col, 12)
	# Incoming artillery: where it comes from, what it will hit
	if not sim.shells.is_empty() and fmod(t * 4.0, 1.0) < 0.6:
		var sh: Entities.Shell = sim.shells[0]
		var rel := Rules.angle_diff(sh.angle, view)
		var arrow := "" if absf(rel) < FOV * 0.6 else ("  >" if rel > 0.0 else "<  ")
		var txt := "INCOMING  %s" % Rules.SECTION_SHORT[sh.target]
		if rel >= FOV * 0.6:
			txt += arrow
		elif rel <= -FOV * 0.6:
			txt = arrow + txt
		Theme2.text(ci, px(W - 6, 30), txt, Theme2.RED, 12, 2)
	# Crosshair
	var cc := px(W / 2.0, H * 0.5)
	for seg in [[Vector2(-10, 0), Vector2(-4, 0)], [Vector2(4, 0), Vector2(10, 0)], [Vector2(0, -10), Vector2(0, -4)], [Vector2(0, 4), Vector2(0, 10)]]:
		ci.draw_line(cc + seg[0], cc + seg[1], Theme2.dim(Theme2.GREEN, 0.7))
	# Off-view target arrow
	var e := sim.target()
	if e:
		var rel := Rules.angle_diff(e.est_angle, view)
		if absf(rel) >= FOV * 0.6 and fmod(t * 3.0, 1.0) < 0.7:
			var right := rel > 0.0
			var ex := W - 8.0 if right else 8.0
			var ey := H / 2.0
			var dir := 1.0 if right else -1.0
			var c := Color8(255, 80, 60)
			ci.draw_colored_polygon(PackedVector2Array([px(ex + 5 * dir, ey), px(ex - 3 * dir, ey - 5), px(ex - 3 * dir, ey + 5)]), c)
			Theme2.text(ci, px(ex - 8 * dir, ey + 4), e.callsign(), c, 10, 2 if right else 0)
	# Label, REC dot, weak optics
	if fmod(t * 1.5, 1.0) < 0.6:
		ci.draw_rect(Rect2(px(6, H - 10), Vector2(3, 3)), Theme2.RED)
	var cam := Rules.camera_for(view)
	Theme2.text(ci, px(12, H - 4), "CAM %s %03d" % [Rules.CAMERA_NAMES[cam], int(Rules.bearing_deg(view))], Theme2.GREEN, 12)
	if mech.camera_ok(cam) and mech.cameras[cam] / Rules.CAMERA_HP < 0.6:
		Theme2.text(ci, px(W - 4, H - 4), "OPTICS %d%%" % int(mech.cameras[cam] / Rules.CAMERA_HP * 100.0), Theme2.AMBER, 12, 2)
	# Static from a damaged head / weak optics, heat tint
	var weak := maxf(1.0 - mech.frac(Rules.Section.SENSORS), 1.0 - mech.cameras[cam] / Rules.CAMERA_HP if mech.camera_ok(cam) else 0.0)
	var n := int(weak * 300.0) + (200 if mech.shake > 0.5 else 0)
	var frame := int(t * 20.0)
	for i in n:
		var h := Rng.hash2(i, frame, 17)
		var v := (40 + (h >> 20) % 200) / 255.0
		ci.draw_rect(Rect2(px(h % int(W), (h >> 9) % int(H)), Vector2(2, 1)), Color(v, v, v))
	if mech.heat > 85.0:
		ci.draw_rect(Rect2(origin, Vector2(W, H)), Color(1, 0.24, 0.08, (mech.heat - 85.0) / 45.0 * 0.35))
	eject_btn = Rect2()
	if mech.overload >= 0.0:
		var r := Rect2(px(W / 2.0 - 100, 28), Vector2(200, 60))
		ci.draw_rect(r, Color(0, 0, 0, 0.85))
		ci.draw_rect(r, Theme2.RED, false, 2.0)
		Theme2.label(ci, px(W / 2.0 - 30, 42), "REACTOR OVERLOAD", Theme2.RED, 8, 1)
		Theme2.text(ci, px(W / 2.0 - 30, 56), "CONTAINMENT IN %.1fs" % maxf(mech.overload, 0.0), Color.WHITE if fmod(t * 4.0, 1.0) < 0.5 else Theme2.RED, 12, 1)
		Theme2.text(ci, px(W / 2.0 - 30, 70), "vent, scram, or get out", Theme2.DIM, 12, 1)
		eject_btn = Rect2(px(W / 2.0 + 44, 34), Vector2(50, 48))
		var hot := fmod(t * 3.0, 1.0) < 0.5
		Theme2.button(ci, eject_btn, "", Theme2.RED, true, hot)
		Pixel.draw(ci, Sprites.EJECT_SEAT, eject_btn.position + Vector2(13, 3), 1.0, false, Color8(30, 10, 10) if hot else Color.WHITE)
		Theme2.label(ci, Vector2(eject_btn.get_center().x, eject_btn.end.y - 4), "EJECT", Color8(30, 10, 10) if hot else Theme2.RED, 8, 1)
