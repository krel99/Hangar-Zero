## The hull diagram: a 40x46 grid where each pixel belongs to a section (or the frame),
## plus a detail overlay (rivets, seams, canopy glass, reactor glow, barrel rings) that
## both the damage display and the hangar portrait shade with.
class_name Hull
extends RefCounted

const W := 40
const H := 46
const FRAME := 255
const EMPTY := 254

enum D { NONE, HI, LO, RIVET, SEAM, GLASS, GLOW, LIGHT, VENT, HAZARD }

static var map: PackedByteArray = _build_map()
static var detail: PackedByteArray = _build_detail()


static func at(x: int, y: int) -> int:
	if x < 0 or y < 0 or x >= W or y >= H:
		return EMPTY
	return map[y * W + x]


static func detail_at(x: int, y: int) -> int:
	return detail[y * W + x]


static func _fill(g: PackedByteArray, x0: int, y0: int, x1: int, y1: int, v: int) -> void:
	for y in range(y0, y1 + 1):
		for x in range(x0, x1 + 1):
			g[y * W + x] = v


static func _build_map() -> PackedByteArray:
	var g := PackedByteArray()
	g.resize(W * H)
	g.fill(EMPTY)
	var S := Rules.Section
	_fill(g, 19, 0, 20, 6, S.UPLINK)
	_fill(g, 16, 1, 23, 2, S.UPLINK)
	_fill(g, 17, 3, 22, 3, S.UPLINK)
	_fill(g, 14, 6, 25, 9, S.SENSORS)
	_fill(g, 12, 7, 13, 8, S.SENSORS)
	_fill(g, 26, 7, 27, 8, S.SENSORS)
	_fill(g, 15, 10, 24, 14, S.COCKPIT)
	_fill(g, 6, 12, 12, 17, S.SHIELD_GEN)
	_fill(g, 27, 12, 33, 17, S.SHIELD_GEN)
	_fill(g, 12, 15, 27, 27, FRAME)
	_fill(g, 13, 16, 14, 26, S.COOLANT)
	_fill(g, 25, 16, 26, 26, S.COOLANT)
	_fill(g, 16, 17, 23, 24, S.REACTOR)
	_fill(g, 3, 18, 10, 29, S.AUTOCANNON)
	_fill(g, 4, 30, 5, 35, S.AUTOCANNON)
	_fill(g, 8, 30, 9, 35, S.AUTOCANNON)
	_fill(g, 29, 18, 36, 28, S.RAILGUN)
	_fill(g, 32, 29, 33, 40, S.RAILGUN)
	_fill(g, 31, 39, 34, 40, S.RAILGUN)
	_fill(g, 14, 28, 25, 30, FRAME)
	_fill(g, 13, 31, 18, 42, S.LEFT_LEG)
	_fill(g, 11, 43, 19, 45, S.LEFT_LEG)
	_fill(g, 21, 31, 26, 42, S.RIGHT_LEG)
	_fill(g, 20, 43, 28, 45, S.RIGHT_LEG)
	return g


static func _put(g: PackedByteArray, x: int, y: int, v: int) -> void:
	g[y * W + x] = v


static func _hline(g: PackedByteArray, x0: int, x1: int, y: int, v: int) -> void:
	for x in range(x0, x1 + 1):
		g[y * W + x] = v


static func _build_detail() -> PackedByteArray:
	var g := PackedByteArray()
	g.resize(W * H)
	g.fill(D.NONE)
	# Uplink: dish ribs and a beacon on top
	_hline(g, 16, 23, 1, D.HI)
	_hline(g, 17, 22, 3, D.LO)
	_put(g, 19, 0, D.LIGHT)
	_put(g, 20, 0, D.LIGHT)
	for y in [2, 4, 6]:
		_put(g, 19, y, D.SEAM)
	# Sensor head: visor band with a red eye, antennae stubs
	_hline(g, 15, 24, 6, D.HI)
	_hline(g, 15, 18, 8, D.SEAM)
	_hline(g, 21, 24, 8, D.SEAM)
	_put(g, 19, 8, D.LIGHT)
	_put(g, 20, 8, D.LIGHT)
	_put(g, 12, 7, D.HI)
	_put(g, 27, 7, D.HI)
	for x in [14, 25]:
		_put(g, x, 9, D.RIVET)
	# Cockpit: canopy glass with a frame, rivets
	_hline(g, 16, 23, 11, D.GLASS)
	_hline(g, 16, 23, 12, D.GLASS)
	for p in [[19, 11], [19, 12], [20, 11], [20, 12]]:
		_put(g, p[0], p[1], D.SEAM)
	_hline(g, 15, 24, 10, D.HI)
	_hline(g, 15, 24, 14, D.LO)
	for x in [15, 24]:
		_put(g, x, 13, D.RIVET)
	# Shield emitters: fin plates
	for x0 in [6, 27]:
		_hline(g, x0, x0 + 6, 12, D.HI)
		_hline(g, x0, x0 + 6, 14, D.SEAM)
		_hline(g, x0, x0 + 6, 16, D.SEAM)
		_put(g, x0 + 3, 13, D.LIGHT)
		_put(g, x0 + 1, 17, D.RIVET)
		_put(g, x0 + 5, 17, D.RIVET)
	# Torso frame: grille and hazard stripes on the hips
	for y in range(15, 28, 2):
		_put(g, 12, y, D.VENT)
		_put(g, 27, y, D.VENT)
	_hline(g, 15, 24, 16, D.SEAM)
	_hline(g, 15, 24, 25, D.SEAM)
	_hline(g, 14, 25, 29, D.HAZARD)
	# Coolant lines: pipe clamps and vents
	for y in range(17, 26, 3):
		_put(g, 13, y, D.LO)
		_put(g, 14, y, D.HI)
		_put(g, 25, y, D.HI)
		_put(g, 26, y, D.LO)
	for y in range(18, 27, 3):
		_put(g, 13, y, D.VENT)
		_put(g, 26, y, D.VENT)
	# Reactor: a ring of bolts around a glowing core
	for p in [[16, 17], [23, 17], [16, 24], [23, 24]]:
		_put(g, p[0], p[1], D.RIVET)
	_hline(g, 17, 22, 18, D.LO)
	_hline(g, 17, 22, 23, D.LO)
	for y in range(19, 23):
		_put(g, 17, y, D.LO)
		_put(g, 22, y, D.LO)
		for x in range(18, 22):
			_put(g, x, y, D.GLOW)
	# Autocannon arm: ammo drum, feed slot, twin barrels with vents
	_hline(g, 3, 10, 18, D.HI)
	_hline(g, 4, 9, 21, D.SEAM)
	_hline(g, 4, 9, 27, D.SEAM)
	for y in range(23, 26):
		_put(g, 5, y, D.LO)
		_put(g, 8, y, D.LO)
	for p in [[3, 19], [10, 19], [3, 28], [10, 28]]:
		_put(g, p[0], p[1], D.RIVET)
	for y in range(31, 36, 2):
		_put(g, 4, y, D.VENT)
		_put(g, 8, y, D.VENT)
	_put(g, 5, 30, D.HI)
	_put(g, 9, 30, D.HI)
	# Railgun: capacitor housing with rails, long barrel with rings and a muzzle
	_hline(g, 29, 36, 18, D.HI)
	_hline(g, 30, 35, 20, D.SEAM)
	for y in range(22, 27):
		_put(g, 31, y, D.GLOW)
		_put(g, 34, y, D.GLOW)
	for p in [[29, 19], [36, 19], [29, 27], [36, 27]]:
		_put(g, p[0], p[1], D.RIVET)
	for y in range(30, 39, 3):
		_put(g, 32, y, D.LO)
		_put(g, 33, y, D.LO)
	for y in range(29, 39):
		if y % 3 != 0:
			_put(g, 32, y, D.HI)
	_hline(g, 31, 34, 39, D.HI)
	_hline(g, 31, 34, 40, D.LO)
	# Legs: hip bolts, knee joint, shin plate, toe lip
	for leg in [[13, 18], [21, 26]]:
		var x0: int = leg[0]
		var x1: int = leg[1]
		_put(g, x0, 31, D.RIVET)
		_put(g, x1, 31, D.RIVET)
		_hline(g, x0, x1, 35, D.SEAM)
		_hline(g, x0 + 1, x1 - 1, 36, D.LO)
		_hline(g, x0, x1, 37, D.SEAM)
		for y in range(32, 43):
			_put(g, x0 + 1, y, D.HI)
		_hline(g, x0 - 1, x1 + 1, 43, D.HI)
		_put(g, x0 + 2, 40, D.VENT)
		_put(g, x0 + 2, 41, D.VENT)
	_hline(g, 11, 19, 45, D.LO)
	_hline(g, 20, 28, 45, D.LO)
	return g


static func _mix(a: Color, b: Color, f: float) -> Color:
	return a.lerp(b, f)


## Apply a detail code to a section's base colour.
static func shade(base: Color, d: int, t: float) -> Color:
	match d:
		D.HI: return base * Color(1.35, 1.35, 1.35)
		D.LO: return base * Color(0.55, 0.55, 0.55)
		D.RIVET: return _mix(base, Color8(20, 22, 26), 0.75)
		D.SEAM: return base * Color(0.42, 0.42, 0.42)
		D.GLASS: return _mix(base, Color8(250, 214, 130), 0.8)
		D.GLOW:
			var pulse := sin(t * 2.5) * 0.5 + 0.5
			return _mix(base, Color8(255, 160, 60), 0.55 + 0.35 * pulse)
		D.LIGHT:
			return Color8(255, 70, 50) if fmod(t * 2.0, 1.0) < 0.5 else _mix(base, Color8(120, 30, 24), 0.7)
		D.VENT: return _mix(base, Color8(16, 16, 20), 0.8)
		D.HAZARD: return _mix(base, Color8(230, 180, 40), 0.7)
	return base


static func hazard_dark(x: int, y: int) -> bool:
	return (((x + y) / 2) % 2) == 0


## Section colour by health, shared by every readout.
static func health_color(f: float) -> Color:
	if f <= 0.0:
		return Color8(70, 30, 30)
	if f < 0.35:
		return Color8(232, 72, 58)
	if f < 0.7:
		return Color8(232, 168, 56)
	return Color8(126, 240, 138)
