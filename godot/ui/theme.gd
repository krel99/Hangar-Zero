## Palette, fonts and the chunky pixel widgets shared by every screen. Designed at the
## 480x270 pixel-perfect viewport; the engine scales it up by whole numbers.
class_name Theme2
extends RefCounted

const BG := Color8(9, 10, 13)
const PANEL := Color8(20, 23, 28)
const PANEL_DARK := Color8(12, 14, 17)
const EDGE_HI := Color8(58, 64, 72)
const EDGE_LO := Color8(5, 6, 8)
const TEXT := Color8(206, 212, 200)
const DIM := Color8(112, 120, 128)
const AMBER := Color8(232, 168, 56)
const GREEN := Color8(126, 240, 138)
const GREEN_DIM := Color8(36, 84, 46)
const RED := Color8(232, 72, 58)
const CYAN := Color8(95, 211, 230)
const PURPLE := Color8(178, 110, 240)
const YELLOW := Color8(242, 200, 74)
const SHIELD := Color8(110, 150, 255)

## True on phones and tablets: keyboard tags and key hints are hidden.
static var touch: bool = OS.has_feature("mobile")

static var pix: Font = load("res://fonts/Silkscreen-Regular.ttf")
static var mono: Font = load("res://fonts/VT323-Regular.ttf")


## Draw a line of pixel-font text. `align`: 0 left, 1 centre, 2 right; `pos.y` is the baseline.
static func label(ci: CanvasItem, pos: Vector2, s: String, color: Color = TEXT, size: int = 8, align: int = 0) -> void:
	var w := pix.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
	var x := pos.x - (w / 2.0 if align == 1 else (w if align == 2 else 0.0))
	ci.draw_string(pix, Vector2(roundf(x), roundf(pos.y)), s, HORIZONTAL_ALIGNMENT_LEFT, -1, size, color)


static func text(ci: CanvasItem, pos: Vector2, s: String, color: Color = TEXT, size: int = 12, align: int = 0) -> void:
	var w := mono.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
	var x := pos.x - (w / 2.0 if align == 1 else (w if align == 2 else 0.0))
	ci.draw_string(mono, Vector2(roundf(x), roundf(pos.y)), s, HORIZONTAL_ALIGNMENT_LEFT, -1, size, color)


static func text_w(s: String, size: int = 12) -> float:
	return mono.get_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x


## Riveted bevelled panel.
static func panel(ci: CanvasItem, r: Rect2) -> void:
	ci.draw_rect(r, PANEL)
	ci.draw_rect(Rect2(r.position, Vector2(r.size.x, 1)), EDGE_HI)
	ci.draw_rect(Rect2(r.position, Vector2(1, r.size.y)), EDGE_HI)
	ci.draw_rect(Rect2(r.position + Vector2(0, r.size.y - 1), Vector2(r.size.x, 1)), EDGE_LO)
	ci.draw_rect(Rect2(r.position + Vector2(r.size.x - 1, 0), Vector2(1, r.size.y)), EDGE_LO)
	for c in [r.position + Vector2(2, 2), r.position + Vector2(r.size.x - 4, 2), r.position + Vector2(2, r.size.y - 4), r.position + Vector2(r.size.x - 4, r.size.y - 4)]:
		ci.draw_rect(Rect2(c, Vector2(2, 2)), EDGE_HI)
		ci.draw_rect(Rect2(c + Vector2(1, 1), Vector2(1, 1)), EDGE_LO)


## Chunky button. `lit` keeps it glowing (toggle state); returns nothing, hit-testing is the
## caller's job (see Cockpit.hit).
static func button(ci: CanvasItem, r: Rect2, s: String, color: Color, enabled: bool, lit: bool, pressed: bool = false) -> void:
	var base := Color8(34, 36, 40) if not enabled else (color if lit else color * Color(0.32, 0.32, 0.32))
	ci.draw_rect(r, EDGE_LO)
	var inner := r.grow(-1)
	ci.draw_rect(inner, base)
	var hi := Color(1, 1, 1, 0.27) if not pressed else Color(0, 0, 0, 0.3)
	var lo := Color(0, 0, 0, 0.47) if not pressed else Color(1, 1, 1, 0.2)
	ci.draw_rect(Rect2(inner.position, Vector2(inner.size.x, 1)), hi)
	ci.draw_rect(Rect2(inner.position, Vector2(1, inner.size.y)), hi)
	ci.draw_rect(Rect2(inner.position + Vector2(0, inner.size.y - 1), Vector2(inner.size.x, 1)), lo)
	ci.draw_rect(Rect2(inner.position + Vector2(inner.size.x - 1, 0), Vector2(1, inner.size.y)), lo)
	var tc := DIM if not enabled else (Color8(14, 12, 10) if lit else color)
	if s != "":
		label(ci, Vector2(r.get_center().x, r.get_center().y + 3), s, tc, 8, 1)


## LED segment bar.
static func seg_bar(ci: CanvasItem, r: Rect2, frac: float, segs: int, on: Color) -> void:
	ci.draw_rect(r, EDGE_LO)
	var gap := 1.0
	var w := (r.size.x - gap * (segs + 1)) / segs
	var lit := int(roundf(clampf(frac, 0.0, 1.0) * segs))
	for i in segs:
		var sr := Rect2(r.position + Vector2(gap + i * (w + gap), gap), Vector2(w, r.size.y - gap * 2))
		ci.draw_rect(sr, on if i < lit else on * Color(0.12, 0.12, 0.12, 1))


static func scanlines(ci: CanvasItem, r: Rect2, alpha: float) -> void:
	var y := r.position.y
	while y < r.end.y:
		ci.draw_rect(Rect2(Vector2(r.position.x, y), Vector2(r.size.x, 1)), Color(0, 0, 0, alpha))
		y += 3.0


static func fmt_clock(t: float) -> String:
	var s := int(maxf(t, 0.0))
	return "%02d:%02d" % [s / 60, s % 60]


static func dim(c: Color, k: float) -> Color:
	return Color(c.r * k, c.g * k, c.b * k, c.a)


static func pulse(t: float, hz: float) -> float:
	return sin(t * hz * TAU) * 0.5 + 0.5


## Wrapped prose. Returns the height used.
static func text_wrap(ci: CanvasItem, pos: Vector2, s: String, w: float, color: Color = TEXT, size: int = 12, max_lines: int = 8) -> float:
	ci.draw_multiline_string(mono, Vector2(roundf(pos.x), roundf(pos.y)), s, HORIZONTAL_ALIGNMENT_LEFT, w, size, max_lines, color)
	var lines := mono.get_multiline_string_size(s, HORIZONTAL_ALIGNMENT_LEFT, w, size, max_lines)
	return lines.y


## Dotted pixel ring (radar range rings, uncertainty arcs).
static func ring(ci: CanvasItem, c: Vector2, r: float, color: Color, dash: bool = false) -> void:
	var n := maxi(int(r * TAU / 2.0), 8)
	for i in n:
		if dash and (i / 3) % 2 == 1:
			continue
		var a := float(i) / n * TAU
		ci.draw_rect(Rect2(Vector2(roundf(c.x + cos(a) * r), roundf(c.y + sin(a) * r)), Vector2.ONE), color)


## 7x7 `x` icon drawn around `c` with `k`-pixel cells.
static func icon7(ci: CanvasItem, c: Vector2, rows: Array, k: float, color: Color) -> void:
	for y in rows.size():
		var row: String = rows[y]
		for x in row.length():
			if row[x] == "x":
				ci.draw_rect(Rect2(Vector2(roundf(c.x + (x - 3.5) * k), roundf(c.y + (y - 3.5) * k)), Vector2(k, k)), color)


## Polar point: bearing 0 is up, clockwise.
static func polar(c: Vector2, angle: float, r: float) -> Vector2:
	return c + Vector2(sin(angle), -cos(angle)) * r


static func sys_color(s: int) -> Color:
	return [Color8(214, 160, 92), GREEN, CYAN, YELLOW, SHIELD, Color8(240, 130, 60), Color8(150, 230, 255)][s]


static func kind_color(k: int) -> Color:
	return [RED, Color8(255, 140, 60), PURPLE][k]


## Truncate prose to fit a width.
static func fit(s: String, w: float, size: int = 12) -> String:
	while s.length() > 1 and text_w(s, size) > w:
		s = s.substr(0, s.length() - 1)
	return s
