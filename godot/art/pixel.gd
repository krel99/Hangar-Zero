## Bakes text-grid sprites into nearest-filtered textures, cached by (grid, accent, flip),
## plus the tiny helpers every procedural painter uses.
class_name Pixel
extends RefCounted

const ACCENT_KEY := "*"
static var _cache: Dictionary = {}


## Width of a grid in pixels (longest row).
static func grid_w(rows: Array) -> int:
	var w := 0
	for r in rows:
		w = maxi(w, r.length())
	return w


## Bake a grid. `accent` colours `*` pixels; `mask` makes an all-white silhouette (for
## flash overlays). `flip` mirrors horizontally.
static func texture(rows: Array, accent: Color = Color(0.91, 0.66, 0.22), flip: bool = false, mask: bool = false) -> ImageTexture:
	var key := [rows, accent, flip, mask]
	if _cache.has(key):
		return _cache[key]
	var w := grid_w(rows)
	var h := rows.size()
	var img := Image.create(w, h, false, Image.FORMAT_RGBA8)
	for y in h:
		var row: String = rows[y]
		for x in row.length():
			var ch := row[x]
			if not Sprites.PALETTE.has(ch):
				continue
			var c: Color = accent if ch == ACCENT_KEY else Sprites.PALETTE[ch]
			if mask:
				c = Color.WHITE
			img.set_pixel(w - 1 - x if flip else x, y, c)
	var tex := ImageTexture.create_from_image(img)
	_cache[key] = tex
	return tex


## Draw a grid at `pos` scaled `k` times. `flash` blends toward white, `tint` multiplies.
static func draw(ci: CanvasItem, rows: Array, pos: Vector2, k: float = 1.0, flip: bool = false, tint: Color = Color.WHITE, flash: float = 0.0, accent: Color = Color(0.91, 0.66, 0.22)) -> void:
	var tex := texture(rows, accent, flip)
	var size := Vector2(tex.get_width(), tex.get_height()) * k
	ci.draw_texture_rect(tex, Rect2(pos, size), false, tint)
	if flash > 0.0:
		var m := texture(rows, accent, flip, true)
		ci.draw_texture_rect(m, Rect2(pos, size), false, Color(1, 1, 1, flash))


static func lerp3(a: Color, b: Color, t: float) -> Color:
	return a.lerp(b, clampf(t, 0.0, 1.0))


## Ordered-dither threshold for a pixel-art gradient look.
static func bayer(x: int, y: int) -> float:
	const M := [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]
	return (float(M[y & 3][x & 3]) + 0.5) / 16.0


static func put(img: Image, x: int, y: int, c: Color) -> void:
	if x >= 0 and y >= 0 and x < img.get_width() and y < img.get_height():
		img.set_pixel(x, y, c)


## Pixel colour, or null if transparent / outside.
static func peek(img: Image, x: int, y: int):
	if x >= 0 and y >= 0 and x < img.get_width() and y < img.get_height():
		var c := img.get_pixel(x, y)
		if c.a > 0.0:
			return c
	return null
