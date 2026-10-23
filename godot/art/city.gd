## Procedural backdrops, painted once at startup: a two-layer 360° panorama of the dying
## city (far layer parallaxes) and the ruined hangar.
class_name City
extends RefCounted

const PANO_W := 1280
const PANO_H := 208
const HORIZON := 124
const HANGAR_W := 400
const HANGAR_H := 225

static var far_tex: ImageTexture
static var near_tex: ImageTexture
static var hangar_tex: ImageTexture


static func ensure() -> void:
	if far_tex == null:
		far_tex = ImageTexture.create_from_image(city_far())
		near_tex = ImageTexture.create_from_image(city_near())
		hangar_tex = ImageTexture.create_from_image(hangar_backdrop())


static func _c(r: int, g: int, b: int) -> Color:
	return Color8(r, g, b)


## Far layer: sky, moon, mountains, distant towers and smoke.
static func city_far() -> Image:
	var w := PANO_W
	var h := PANO_H
	var img := Image.create(w, h, false, Image.FORMAT_RGBA8)
	var sky_top := _c(18, 14, 30)
	var sky_mid := _c(96, 42, 56)
	var sky_low := _c(206, 104, 56)
	for y in h:
		for x in w:
			var t := float(y) / HORIZON
			var c: Color
			if t < 0.55:
				var tt := t / 0.55
				c = sky_top.lerp(sky_mid, floorf(tt * 6.0 + Pixel.bayer(x, y) * 0.9) / 6.0)
			else:
				var tt := (t - 0.55) / 0.45
				c = sky_mid.lerp(sky_low, floorf(tt * 7.0 + Pixel.bayer(x, y) * 0.9) / 7.0)
			img.set_pixel(x, y, c)
	# Stars
	for i in 320:
		var x := Rng.hash2(i, 1, 7) % w
		var y := Rng.hash2(i, 2, 7) % 46
		var bright := Rng.hash2(i, 3, 7) % 4 == 0
		Pixel.put(img, x, y, _c(220, 210, 230) if bright else _c(150, 140, 170))
		if bright:
			Pixel.put(img, x + 1, y, _c(110, 100, 130))
	# Moon, cratered and sickly, with a halo
	var mx := 300
	var my := 28
	for dy in range(-10, 11):
		for dx in range(-10, 11):
			var d2 := dx * dx + dy * dy
			if d2 <= 100:
				var c := _c(176, 158, 126) if d2 > 72 else _c(230, 212, 168)
				if Rng.hashf(dx, dy, 44) > 0.86:
					c = _c(190, 170, 130)
				if (dx - 3) * (dx - 3) + (dy + 2) * (dy + 2) < 5 or (dx + 4) * (dx + 4) + (dy - 3) * (dy - 3) < 3:
					c = _c(168, 150, 118)
				Pixel.put(img, mx + dx, my + dy, c)
			elif d2 <= 196 and Pixel.bayer(dx, dy) < 0.25:
				Pixel.put(img, mx + dx, my + dy, _c(96, 62, 70))
	# Mountains, two ridges
	for ridge in [[11, HORIZON - 10, 22.0, _c(60, 36, 58), _c(84, 52, 72)], [29, HORIZON - 4, 14.0, _c(46, 28, 46), _c(62, 38, 58)]]:
		var seed_v: int = ridge[0]
		var base: int = ridge[1]
		var amp: float = ridge[2]
		for x in w:
			var fx := float(x) / w * TAU
			var hgt := sin(fx * 3.0 + seed_v) * 0.5 + sin(fx * 7.0 + seed_v * 1.7) * 0.3 + sin(fx * 19.0) * 0.2
			var ih := int(hgt * amp + amp * 0.6) + Rng.hash2(x / 3, 0, seed_v) % 3
			for y in range(base - ih, HORIZON):
				Pixel.put(img, x, y, ridge[4] if y < base - ih + 2 else ridge[3])
	# Smoke columns
	for col in 7:
		var cx := Rng.hash2(col, 9, 3) % w
		for y in range(8, HORIZON):
			var spread := int((HORIZON - y) * 0.28) + 2
			var drift := sin(y * 0.11) * 7.0 + (HORIZON - y) * 0.35
			for dx in range(-spread, spread + 1):
				var edge := float(absi(dx)) / spread
				if Rng.hashf(cx + dx, y, 11) > 0.42 + edge * 0.5:
					var shade := _c(62, 44, 50) if Rng.hashf(cx + dx, y, 12) > 0.7 else _c(44, 30, 38)
					Pixel.put(img, cx + dx + int(drift), y, shade)
	# Far skyline: dense dead towers, some with a single lit floor
	var far := _c(48, 30, 46)
	var far_hi := _c(64, 42, 60)
	var x := 0
	while x < w:
		var bw := 6 + Rng.hash2(x, 3, 1) % 14
		var bh := 16 + Rng.hash2(x, 4, 1) % 40
		var broken := Rng.hash2(x, 5, 1) % 3 == 0
		var lit := Rng.hash2(x, 6, 1) % 5 == 0
		for bx in range(x, mini(x + bw, w)):
			var top := HORIZON - bh
			if broken:
				top += int((bx - x) * 0.9) % 9
			for y in range(top, HORIZON):
				var c := far
				if bx == x:
					c = far_hi
				elif lit and y == top + 6 and (bx - x) % 2 == 0:
					c = _c(150, 110, 70)
				Pixel.put(img, bx, y, c)
		if Rng.hash2(x, 7, 1) % 4 == 0:
			var mast := x + bw / 2
			for y in range(HORIZON - bh - 12, HORIZON - bh):
				Pixel.put(img, mast, y, far)
			Pixel.put(img, mast, HORIZON - bh - 13, _c(230, 60, 40))
		x += bw + Rng.hash2(x, 6, 1) % 4
	# The dead orbital tether: one colossal tower, visible from everywhere
	for y in HORIZON:
		var hw := 2 + y / 30
		for dx in range(-hw, hw + 1):
			var c := _c(40, 26, 42)
			if dx == -hw:
				c = _c(70, 46, 66)
			elif (y / 6) % 2 == 0 and dx == 0:
				c = _c(58, 36, 54)
			Pixel.put(img, 900 + dx, y, c)
		if y % 18 == 0:
			Pixel.put(img, 900 - hw - 1, y, _c(255, 90, 60))
			Pixel.put(img, 900 + hw + 1, y, _c(255, 90, 60))
	return img


## Near layer: ruined buildings, cranes, pylons with wires, the street and its rubble.
## Transparent above the roofline so the far layer shows through.
static func city_near() -> Image:
	var w := PANO_W
	var h := PANO_H
	var img := Image.create(w, h, false, Image.FORMAT_RGBA8)
	img.fill(Color(0, 0, 0, 0))
	var wall := _c(26, 20, 28)
	var wall_lit := _c(40, 30, 40)
	var edge_c := _c(16, 12, 20)
	var black := Color.BLACK
	var x := 3
	while x < w:
		var bw := 12 + Rng.hash2(x, 7, 2) % 26
		var bh := 22 + Rng.hash2(x, 8, 2) % 50
		var style := Rng.hash2(x, 9, 2) % 5
		var win_w := 2 + Rng.hash2(x, 10, 2) % 2
		var pitch := 4 + Rng.hash2(x, 11, 2) % 2
		for bx in range(x, mini(x + bw, w)):
			var lx := bx - x
			var top := HORIZON - bh
			match style:
				0: top += mini(lx * lx / 6, bh / 2)
				1: top += 0 if (lx / 3) % 2 == 0 else 3
				2: top += int(absf(sin(lx * 0.7)) * 6.0)
				3: top += (lx - bw / 2) * 2 if lx > bw / 2 else 0
			var face := wall_lit if lx > bw * 2 / 3 else wall
			for y in range(top, HORIZON):
				var c := face
				if lx == 0 or lx == bw - 1:
					c = edge_c
				if (y - top) % 7 == 6 and lx > 0 and lx < bw - 1:
					c = c.lerp(black, 0.35)
				var wx := lx % pitch
				var wy := (y - top) % 7
				if wx >= 1 and wx <= win_w and wy >= 2 and wy <= 4 and y < HORIZON - 5 and lx > 1 and lx < bw - 2:
					var r := Rng.hash2(bx - wx, y - wy, 13) % 29
					if r == 0:
						c = _c(250, 196, 90)
					elif r == 1:
						c = _c(236, 128, 50)
					elif r == 2:
						c = _c(120, 190, 210)
					elif r <= 5:
						c = _c(62, 46, 58)
					elif r <= 8:
						c = _c(16, 12, 18)
					else:
						c = _c(34, 26, 36)
					if wy == 4 and r > 2:
						c = c.lerp(black, 0.4)
				Pixel.put(img, bx, y, c)
			# Fires on the roofline and in blown-out floors
			if Rng.hash2(bx, 1, 21) % 41 == 0:
				for fy in 3 + Rng.hash2(bx, 2, 21) % 6:
					var c := _c(252, 220, 110) if fy < 2 else (_c(236, 120, 46) if fy < 4 else _c(170, 60, 30))
					Pixel.put(img, bx, top - fy, c)
					Pixel.put(img, bx + 1, top - fy + 1, _c(210, 80, 34))
					Pixel.put(img, bx - 1, top - fy + 2, _c(120, 40, 30))
		# Rooftop clutter: water tanks, aerials, a crane
		var roof := HORIZON - bh
		var clutter := Rng.hash2(x, 12, 2) % 4
		if clutter == 0:
			for y in range(roof - 6, roof):
				for dx in range(2, 6):
					Pixel.put(img, x + dx, y, _c(52, 40, 48) if y == roof - 6 else _c(30, 22, 30))
		elif clutter == 1:
			var mx := x + bw / 2
			for y in range(roof - 10, roof):
				Pixel.put(img, mx, y, wall)
				if y % 3 == 0:
					Pixel.put(img, mx - 1, y, wall)
					Pixel.put(img, mx + 1, y, wall)
			Pixel.put(img, mx, roof - 11, _c(255, 60, 40))
		elif clutter == 2 and bw > 20:
			var cx := x + 4
			for y in range(roof - 28, roof):
				Pixel.put(img, cx, y, _c(34, 28, 30))
				Pixel.put(img, cx + 1, y, _c(22, 18, 22))
				if y % 4 == 0:
					Pixel.put(img, cx + 2, y, _c(34, 28, 30))
			for dx in range(-6, 23):
				Pixel.put(img, cx + dx, roof - 28, _c(34, 28, 30))
				if dx % 3 == 0:
					Pixel.put(img, cx + dx, roof - 27, _c(22, 18, 22))
			for y in range(roof - 27, roof - 14):
				Pixel.put(img, cx + 18, y, _c(22, 18, 22))
			Pixel.put(img, cx + 18, roof - 14, _c(120, 90, 30))
			Pixel.put(img, cx + 18, roof - 13, _c(120, 90, 30))
		x += bw + 6 + Rng.hash2(x, 10, 2) % 20
	# Ground: cracked asphalt with perspective bands
	for y in range(HORIZON, h):
		var t := float(y - HORIZON) / (h - HORIZON)
		for gx in w:
			var c := _c(38, 30, 34).lerp(_c(66, 54, 48), clampf(t + (Pixel.bayer(gx, y) - 0.5) * 0.25, 0.0, 1.0))
			var n := Rng.hashf(gx / 2, y, 31)
			if n > 0.94:
				c = _c(88, 74, 62)
			elif n < 0.04:
				c = _c(22, 18, 20)
			if Rng.hashf(gx / 5 + y / 3, 0, 37) > 0.93 and Rng.hashf(gx, y, 38) > 0.4:
				c = _c(20, 16, 18)
			img.set_pixel(gx, y, c)
	for gx in w:
		Pixel.put(img, gx, HORIZON + 6, _c(52, 44, 40))
		Pixel.put(img, gx, HORIZON + 7, _c(30, 24, 26))
	# Power pylons with sagging wires
	var px := 60
	while px < w:
		var top := HORIZON - 42
		for y in range(top, HORIZON + 4):
			Pixel.put(img, px, y, _c(30, 26, 30))
			Pixel.put(img, px + 1, y, _c(22, 18, 22))
			if (y - top) % 6 == 0 and y < HORIZON - 4:
				for dx in range(-4, 6):
					Pixel.put(img, px + dx, y, _c(30, 26, 30))
		var next := px + 130
		if next < w:
			for i in 131:
				var f := i / 130.0
				var sag := f * (1.0 - f) * 26.0
				for arm in [[0, -4], [6, 5]]:
					Pixel.put(img, px + arm[1] + i, top + arm[0] + int(sag), _c(18, 14, 18))
		px = next
	# Street level: lamp posts, wrecked cars, barricades, rubble
	for i in 60:
		var cx := Rng.hash2(i, 40, 5) % w
		var depth := Rng.hash2(i, 41, 5) % 40
		var cy := HORIZON + 4 + depth
		var kind := Rng.hash2(i, 42, 5) % 5
		var k := 1 + depth / 20
		if kind <= 1:
			var r := (3 + Rng.hash2(i, 43, 5) % 6) * k
			for dy in range(-r, 1):
				for dx in range(-r * 2, r * 2 + 1):
					if (dx * dx) / 4 + dy * dy <= r * r and Rng.hashf(cx + dx, cy + dy, 43) > 0.2:
						var c := _c(98, 86, 74) if dy < -r / 2 else (_c(140, 90, 50) if Rng.hashf(cx + dx, dy, 45) > 0.85 else _c(58, 48, 44))
						Pixel.put(img, cx + dx, cy + dy, c)
		elif kind == 2:
			var cw := 9 * k
			var ch := 4 * k
			for dy in range(-ch, 1):
				for dx in cw:
					var roof := dy < -ch / 2 and dx > cw / 5 and dx < cw * 4 / 5
					var body := dy >= -ch / 2
					if roof or body:
						var c := _c(70, 60, 60) if dy == -ch / 2 else (_c(120, 70, 40) if Rng.hashf(dx, dy, i) > 0.8 else _c(46, 40, 44))
						Pixel.put(img, cx + dx, cy + dy, c)
			Pixel.put(img, cx + 1, cy + 1, _c(20, 18, 20))
			Pixel.put(img, cx + cw - 2, cy + 1, _c(20, 18, 20))
		elif kind == 3:
			var lit := Rng.hash2(i, 44, 5) % 3 == 0
			var hgt := 18 * k
			for y in range(cy - hgt, cy):
				Pixel.put(img, cx, y, _c(40, 36, 40))
			Pixel.put(img, cx + 1, cy - hgt, _c(40, 36, 40))
			Pixel.put(img, cx + 2, cy - hgt, _c(40, 36, 40))
			Pixel.put(img, cx + 2, cy - hgt + 1, _c(255, 230, 150) if lit else _c(60, 56, 60))
			if lit:
				for dy in range(2, 6):
					for dx in range(-dy, dy + 1):
						if Pixel.bayer(cx + dx, cy - hgt + dy) < 0.3:
							var c = Pixel.peek(img, cx + 2 + dx, cy - hgt + 1 + dy)
							if c != null:
								Pixel.put(img, cx + 2 + dx, cy - hgt + 1 + dy, c.lerp(_c(255, 220, 140), 0.35))
		else:
			for dy in range(-3 * k, 1):
				for dx in 7 * k:
					var stripe := ((dx + dy) / (2 * k)) % 2 == 0 and dy > -2 * k
					Pixel.put(img, cx + dx, cy + dy, _c(170, 130, 40) if stripe else _c(86, 80, 76))
	# Puddles reflecting the sky
	for i in 18:
		var cx := Rng.hash2(i, 50, 5) % w
		var cy := HORIZON + 14 + Rng.hash2(i, 51, 5) % 28
		var r := 3 + Rng.hash2(i, 52, 5) % 5
		for dy in range(-1, 2):
			for dx in range(-r * 2, r * 2 + 1):
				if (dx * dx) / 4 + dy * dy * 4 <= r * r:
					Pixel.put(img, cx + dx, cy + dy, _c(118, 66, 60) if Pixel.bayer(dx, dy) < 0.5 else _c(70, 44, 52))
	return img


## The ruined hangar: trusses, pipes, a broken skylight, gantries and junk.
static func hangar_backdrop() -> Image:
	var w := HANGAR_W
	var h := HANGAR_H
	var img := Image.create(w, h, false, Image.FORMAT_RGBA8)
	var floor_y := 176
	var black := Color.BLACK
	for y in h:
		for x in w:
			img.set_pixel(x, y, _c(12, 12, 18).lerp(_c(36, 32, 34), clampf(float(y) / h + (Pixel.bayer(x, y) - 0.5) * 0.12, 0.0, 1.0)))
	# Back wall: corrugated panels, stains, rust streaks
	for y in range(36, floor_y):
		for x in w:
			var c := img.get_pixel(x, y)
			if x % 40 == 0 or x % 40 == 1:
				c = _c(20, 20, 26)
			elif y % 26 == 10:
				c = _c(22, 22, 28)
			elif (x / 4) % 2 == 0:
				c = c.lerp(black, 0.12)
			if Rng.hashf(x, y, 3) > 0.988:
				c = _c(62, 42, 30)
			if Rng.hashf(x, 0, 4) > 0.9 and y < 36 + int(Rng.hashf(x, 1, 4) * 60.0) and Rng.hashf(x, y, 5) > 0.3:
				c = c.lerp(_c(96, 58, 36), 0.5)
			img.set_pixel(x, y, c)
	# Painted bay number and warning line
	var glyph_9 := ["xxxx", "x..x", "x..x", "xxxx", "...x", "...x", "xxxx"]
	for gy in glyph_9.size():
		for gx in 4:
			if glyph_9[gy][gx] == "x":
				for dy in 5:
					for dx in 5:
						Pixel.put(img, 40 + gx * 5 + dx, 60 + gy * 5 + dy, _c(120, 100, 50))
	for x in range(20, 90):
		Pixel.put(img, x, 100, _c(110, 90, 44))
		Pixel.put(img, x, 101, _c(80, 64, 30))
	# Pipes along the wall with valves
	for pipe in [[44, _c(66, 58, 50)], [48, _c(48, 44, 40)], [128, _c(60, 48, 44)]]:
		var py: int = pipe[0]
		var col: Color = pipe[1]
		for x in w:
			Pixel.put(img, x, py, col)
			Pixel.put(img, x, py + 1, col.lerp(black, 0.4))
			if x % 60 == 30:
				for dy in range(-2, 4):
					Pixel.put(img, x, py + dy, col.lerp(Color.WHITE, 0.2))
					Pixel.put(img, x + 1, py + dy, col.lerp(black, 0.3))
			if x % 120 == 75:
				for dy in range(-3, 1):
					Pixel.put(img, x, py + dy, _c(200, 60, 40))
					Pixel.put(img, x + 1, py + dy, _c(140, 40, 30))
	# Conduit dropping to a fuse box
	for y in range(50, 96):
		Pixel.put(img, 300, y, _c(48, 44, 40))
	for y in range(96, 116):
		for x in range(294, 308):
			Pixel.put(img, x, y, _c(70, 66, 60) if (x == 294 or y == 96) else _c(40, 38, 36))
	Pixel.put(img, 304, 100, _c(120, 240, 130))
	Pixel.put(img, 298, 100, _c(230, 60, 40))
	# Roof trusses
	for x in w:
		for y in [14, 15, 32, 33]:
			Pixel.put(img, x, y, _c(56, 54, 60))
		var m := x % 28
		var y := 16 + (m * 16 / 14 if m < 14 else (28 - m) * 16 / 14)
		Pixel.put(img, x, mini(y, 31), _c(46, 44, 50))
		Pixel.put(img, x, mini(y + 1, 31), _c(30, 28, 34))
		if x % 28 == 0:
			for yy in range(16, 32):
				Pixel.put(img, x, yy, _c(46, 44, 50))
	# Floodlights hanging from the trusses, two dead, one working with a cone
	for light in [[70, false], [200, true], [330, false]]:
		var lx: int = light[0]
		var on: bool = light[1]
		for y in range(34, 42):
			Pixel.put(img, lx, y, _c(40, 38, 40))
		for dx in range(-5, 6):
			for dy in 4:
				var c := _c(60, 58, 60)
				if dy == 3:
					c = _c(255, 240, 200) if on else _c(50, 48, 50)
				Pixel.put(img, lx + dx, 42 + dy, c)
		if on:
			for y in range(46, floor_y + 20):
				var half := 6 + (y - 46) * 2 / 5
				for dx in range(-half, half + 1):
					var f := 1.0 - float(absi(dx)) / half
					if Pixel.bayer(lx + dx, y) < 0.3 * f * (1.0 - (y - 46) / 200.0):
						var c = Pixel.peek(img, lx + dx, y)
						if c != null:
							Pixel.put(img, lx + dx, y, c.lerp(_c(200, 190, 160), 0.28))
	# Broken skylight with light shafts
	for y in 14:
		for x in range(150, 250):
			if Rng.hashf(x / 3, y / 2, 5) > 0.3:
				Pixel.put(img, x, y, _c(74, 80, 100))
			if x % 25 == 0 or y == 13:
				Pixel.put(img, x, y, _c(40, 42, 52))
	for y in range(14, h):
		for x in w:
			var center := 200.0 + (y - 14.0) * 0.32
			var d := absf(x - center)
			var half := 36.0 + y * 0.2
			if d < half and Pixel.bayer(x, y) < 0.2 * (1.0 - d / half) + 0.03:
				img.set_pixel(x, y, img.get_pixel(x, y).lerp(_c(150, 150, 172), 0.25))
	# Floor: plates, grating strip, oil stains, painted lines
	for y in range(floor_y, h):
		for x in w:
			var t := float(y - floor_y) / (h - floor_y)
			var c := _c(36, 32, 32).lerp(_c(54, 46, 42), t)
			if (x + (y - floor_y) * 2) % 48 < 2:
				c = _c(72, 62, 32)
			if (y - floor_y) % 12 == 11:
				c = c.lerp(black, 0.4)
			if Rng.hashf(x / 3, y / 3, 61) > 0.96:
				c = c.lerp(_c(10, 8, 10), 0.5)
			img.set_pixel(x, y, c)
	for x in w:
		Pixel.put(img, x, floor_y, _c(84, 80, 78))
		Pixel.put(img, x, floor_y + 1, _c(40, 36, 36))
		for y in range(h - 14, h - 10):
			Pixel.put(img, x, y, _c(204, 164, 44) if ((x + y) / 4) % 2 == 0 else _c(22, 22, 22))
	# Scaffolding gantries with railings and a catwalk
	for gx in [118, 282]:
		for y in range(50, floor_y):
			Pixel.put(img, gx, y, _c(96, 84, 52))
			Pixel.put(img, gx + 1, y, _c(64, 56, 36))
			Pixel.put(img, gx + 14, y, _c(96, 84, 52))
			Pixel.put(img, gx + 15, y, _c(64, 56, 36))
			if y % 24 == 0:
				for x in range(gx - 18, gx + 16):
					Pixel.put(img, x, y, _c(96, 84, 52))
					Pixel.put(img, x, y + 1, _c(50, 44, 30))
					Pixel.put(img, x, y - 6, _c(80, 70, 44))
				for x in range(gx - 18, gx + 16, 4):
					for dy in range(-6, 0):
						Pixel.put(img, x, y + dy, _c(80, 70, 44))
			var m := (y - 50) % 24
			Pixel.put(img, gx + 2 + m * 12 / 24, y, _c(70, 60, 40))
			Pixel.put(img, gx + 13 - m * 12 / 24, y, _c(70, 60, 40))
	# Overhead crane rail with hook and chains
	for x in range(100, 300):
		Pixel.put(img, x, 34, _c(80, 78, 84))
		Pixel.put(img, x, 35, _c(44, 42, 48))
	for y in range(36, 70):
		Pixel.put(img, 262, y, _c(90, 88, 92) if y % 2 == 0 else _c(60, 58, 64))
		Pixel.put(img, 263, y, _c(40, 40, 44))
	for p in [[0, 70], [1, 71], [2, 72], [2, 73], [1, 74], [0, 75], [-1, 75], [-2, 74], [-2, 73]]:
		Pixel.put(img, 262 + p[0], p[1], _c(200, 160, 40))
	for y in range(36, 60):
		Pixel.put(img, 140, y, _c(70, 68, 72) if y % 2 == 0 else _c(48, 46, 50))
	# Junk: barrels, crates, a tool chest, tyres
	for i in 24:
		var cx := Rng.hash2(i, 1, 77) % w
		if cx >= 130 and cx < 270:
			continue
		match Rng.hash2(i, 2, 77) % 4:
			0:
				var off := Rng.hash2(i, 3, 77) % 10
				for dy in 12:
					for dx in 8:
						var c := _c(110, 90, 40)
						if dy == 0 or dy == 11:
							c = _c(70, 66, 60)
						elif dx == 0:
							c = _c(80, 70, 50)
						elif dx == 7:
							c = _c(40, 34, 28)
						if dy == 3 or dy == 8:
							c = _c(60, 54, 40)
						if Rng.hashf(cx + dx, dy, 78) > 0.85:
							c = _c(96, 58, 36)
						Pixel.put(img, cx + dx, floor_y - 12 + dy + off, c)
			1:
				var s := 8 + Rng.hash2(i, 3, 77) % 6
				for dy in s:
					for dx in s:
						var c := _c(86, 58, 34)
						if dy == 0 or dx == 0:
							c = _c(120, 84, 50)
						elif dy == s - 1 or dx == s - 1:
							c = _c(50, 34, 20)
						elif dx == dy or dx == s - 1 - dy:
							c = _c(96, 66, 40)
						Pixel.put(img, cx + dx, floor_y - s + dy + 2, c)
			2:
				for dy in 14:
					for dx in 18:
						var c := _c(160, 50, 40)
						if dy == 0:
							c = _c(200, 70, 50)
						elif dy % 4 == 3:
							c = _c(60, 20, 16)
						elif dx == 16 and dy % 4 == 1:
							c = _c(220, 200, 160)
						Pixel.put(img, cx + dx, floor_y - 14 + dy + 3, c)
			_:
				for t in 3:
					for dx in 10:
						var c := _c(30, 30, 34)
						if dx == 0 or dx == 9:
							c = _c(20, 20, 22)
						elif dx == 4 or dx == 5:
							c = _c(44, 44, 48)
						for dy in 3:
							Pixel.put(img, cx + dx, floor_y - 2 - t * 3 - dy + 2, c)
	# Welding cables on the floor
	for x in w:
		var y := floor_y + 24 + int(sin(x * 0.05) * 4.0) + int(sin(x * 0.013) * 6.0)
		Pixel.put(img, x, y, _c(22, 20, 22))
		Pixel.put(img, x, y + 1, _c(40, 36, 38))
	return img
