## Between operations: the ruined hangar, the energy matrix, five salvage choices and
## the slow transformation of the machine. Phone layout, 480x270.
class_name HangarView
extends RefCounted

const REPORT := Rect2(4, 20, 126, 92)
const MATRIX := Rect2(334, 20, 142, 92)
const MECH_ORIGIN := Vector2(190, 20)
const MECH_K := 2.0
const CRADLE_Y := 112.0
const CARDS := Rect2(4, 124, 472, 130)
const CARD_GAP := 3.0
const DEPLOY_BTN := Rect2(340, 255, 136, 14)

var campaign: Campaign
var chosen := -1
## "deploy" once the pilot walks.
var request := ""
var t := 0.0

var _matrix_btns: Array[Rect2] = []
var _install_btns: Array[Rect2] = []


func set_campaign(c: Campaign) -> void:
	campaign = c
	chosen = -1
	request = ""


func draw(ci: CanvasItem, now: float) -> void:
	t = now
	City.ensure()
	ci.draw_texture_rect(City.hangar_tex, Rect2(0, 0, 480, 270), false, Color(0.8, 0.8, 0.8))
	_beacon(ci)
	_motes(ci)
	_cradle(ci)
	_mech(ci)
	_oda(ci)
	Theme2.scanlines(ci, Rect2(0, 0, 480, 270), 0.07)
	# Title
	ci.draw_rect(Rect2(4, 2, 472, 14), Color(0, 0, 0, 0.78))
	ci.draw_rect(Rect2(4, 2, 2, 14), Theme2.AMBER)
	Theme2.label(ci, Vector2(10, 12), "THE HANGAR", Theme2.AMBER, 8)
	Theme2.text(ci, Vector2(100, 12), "Depot 9. Rebuild what you can; the rest rusts.", Theme2.TEXT, 12)
	_report(ci)
	_matrix(ci)
	# Identity under the cradle
	Theme2.text(ci, Vector2(476, 121), "%d%% ORIGINAL: %s" % [int(campaign.originality), campaign.identity()], Theme2.AMBER, 12, 2)
	_cards(ci)
	var label := "DEPLOY: OP %02d" % campaign.mission if chosen >= 0 else "SKIP SALVAGE & DEPLOY"
	Theme2.button(ci, DEPLOY_BTN, label, Theme2.AMBER, true, chosen >= 0 and fmod(t * 1.5, 1.0) < 0.7)


func _beacon(ci: CanvasItem) -> void:
	var b := Vector2(118 * 1.2, 46 * 1.2)
	var ang := fposmod(t * 1.4, TAU)
	var dir := Vector2(cos(ang), sin(ang) * 0.35)
	var tip := b + dir * 80.0
	var side := Vector2(-dir.y, dir.x) * 12.0
	ci.draw_colored_polygon(PackedVector2Array([b, tip + side, tip - side]), Color8(255, 140, 40, int((sin(ang) * 0.5 + 0.5) * 28) + 6))
	ci.draw_rect(Rect2(b - Vector2(1, 1), Vector2(2, 2)), Color8(255, 160, 60))


func _motes(ci: CanvasItem) -> void:
	for i in 40:
		var h := Rng.hash2(i, 2, 53)
		var fx := ((h % 100) / 100.0 - 0.5) * 2.0
		var y := fmod(((h >> 8) % 1000) / 1000.0 * 180.0 + t * 2.5 * (1.0 + (h % 3) * 0.3), 170.0) * 1.2
		var x := (200.0 + (y / 1.2 - 14.0) * 0.32) * 1.2 + fx * (36.0 + y * 0.2) + sin(t * 0.8 + i) * 3.0
		ci.draw_rect(Rect2(Vector2(x, y).round(), Vector2.ONE), Color8(230, 225, 210, 70))


func _cradle(ci: CanvasItem) -> void:
	var x0 := MECH_ORIGIN.x - 24.0
	var x1 := MECH_ORIGIN.x + Hull.W * MECH_K + 24.0
	var deck := Rect2(x0, CRADLE_Y, x1 - x0, 3)
	for x in [x0 + 4.0, x1 - 6.0, (x0 + x1) / 2.0 - 1.0]:
		ci.draw_rect(Rect2(x, deck.end.y, 2, 270 - deck.end.y), Color8(52, 48, 40))
	var cy := deck.end.y + 5.0
	while cy < 270.0:
		ci.draw_line(Vector2(x0 + 4, cy), Vector2((x0 + x1) / 2.0, cy + 7), Color8(44, 40, 34))
		ci.draw_line(Vector2(x1 - 4, cy), Vector2((x0 + x1) / 2.0, cy + 7), Color8(44, 40, 34))
		cy += 14.0
	ci.draw_rect(deck, Color8(70, 66, 60))
	var x := x0
	var i := 0
	while x < x1:
		ci.draw_rect(Rect2(x, deck.end.y - 1, minf(2.0, x1 - x), 1), Color8(200, 160, 40) if i % 2 == 0 else Color8(24, 22, 20))
		x += 2.0
		i += 1
	ci.draw_rect(Rect2(deck.position, Vector2(deck.size.x, 1)), Color8(120, 116, 104))
	# Shadow under the feet
	ci.draw_rect(Rect2(MECH_ORIGIN.x - 6, CRADLE_Y - 2, Hull.W * MECH_K + 12, 2), Color(0, 0, 0, 0.6))


func _mech(ci: CanvasItem) -> void:
	for y in Hull.H:
		for x in Hull.W:
			var v := Hull.at(x, y)
			if v == Hull.EMPTY:
				continue
			var base: Color
			if v == Hull.FRAME:
				base = Color8(70, 74, 80)
			else:
				var maker: int = campaign.makers[v]
				var original := maker == Campaign.Maker.BASTION
				base = Color8(96, 100, 106).lerp(Campaign.MAKER_ACCENT[maker], 0.25 if original else 0.7)
				var lost: float = maxf(100.0 - campaign.max_hp[v], 0.0) / 60.0
				if (Rng.hash2(x, y, 12) % 100) / 100.0 < lost:
					base = Color8(110, 60, 36)
				if v == Rules.Section.UPLINK and y < 3 and fmod(t * 2.0, 1.0) < 0.5:
					base = Theme2.CYAN
			var edge := Hull.at(x - 1, y) != v or Hull.at(x + 1, y) != v or Hull.at(x, y - 1) != v or Hull.at(x, y + 1) != v
			var hi := Hull.at(x, y - 1) != v or Hull.at(x - 1, y) != v
			var d := Hull.detail_at(x, y)
			var col: Color
			if d == Hull.D.HAZARD:
				col = Color8(24, 22, 20) if Hull.hazard_dark(x, y) else Hull.shade(base, d, t)
			elif d != Hull.D.NONE:
				col = Hull.shade(base, d, t)
			elif edge:
				col = Theme2.dim(base, 1.2) if hi else Theme2.dim(base, 0.4)
			else:
				col = Theme2.dim(base, 0.8)
			ci.draw_rect(Rect2(MECH_ORIGIN + Vector2(x, y) * MECH_K, Vector2(MECH_K, MECH_K)), col)


func _oda(ci: CanvasItem) -> void:
	var foot := Vector2(MECH_ORIGIN.x - 16, CRADLE_Y - 18)
	Pixel.draw(ci, Sprites.PILOT, foot, 1.0, false, Color8(170, 170, 190))
	var torch := foot + Vector2(9, 8)
	if int(t * 23.0) % 3 != 0:
		ci.draw_rect(Rect2(torch - Vector2(1, 1), Vector2(2, 2)), Color.WHITE)
		ci.draw_rect(Rect2(torch - Vector2(3, 3), Vector2(6, 6)), Color8(150, 200, 255, 50))
		for i in 6:
			var h := Rng.hash2(i, int(t * 20.0), 3)
			var dx := ((h % 100) / 100.0 - 0.3) * 10.0
			var dy := ((h >> 8) % 100) / 100.0 * 7.0
			ci.draw_rect(Rect2((torch + Vector2(dx, dy)).round(), Vector2.ONE), Theme2.YELLOW if h % 2 == 0 else Color8(255, 170, 70))


func _report(ci: CanvasItem) -> void:
	var r := REPORT
	Theme2.panel(ci, r)
	var rep := campaign.last_report
	var y := r.position.y + 22.0
	if rep.is_empty():
		Theme2.label(ci, r.position + Vector2(6, 11), "FIRST SORTIE", Theme2.AMBER, 8)
		Theme2.text(ci, Vector2(r.position.x + 6, y), "No report yet.", Theme2.DIM, 12)
		y += 12.0
	else:
		Theme2.label(ci, r.position + Vector2(6, 11), "OP %02d REPORT" % rep.number, Theme2.AMBER, 8)
		for kv in [["TIME", Theme2.fmt_clock(rep.time)], ["KILLS", str(rep.kills)], ["PEAK HEAT", str(int(rep.heat_peak))]]:
			Theme2.text(ci, Vector2(r.position.x + 6, y), kv[0], Theme2.DIM, 12)
			Theme2.text(ci, Vector2(r.end.x - 6, y), kv[1], Theme2.TEXT, 12, 2)
			y += 10.0
		y += 2.0
		Theme2.label(ci, Vector2(r.position.x + 6, y), "SCARS", Theme2.RED, 8)
		y += 10.0
		var scars: Array = rep.scars
		if scars.is_empty():
			Theme2.text(ci, Vector2(r.position.x + 6, y), "none. A clean run.", Theme2.DIM, 12)
			y += 10.0
		for sc in scars.slice(0, 3):
			Theme2.text(ci, Vector2(r.position.x + 6, y), Rules.SECTION_NAME[sc[0]], Color8(230, 140, 120), 12)
			Theme2.text(ci, Vector2(r.end.x - 6, y), "-%d" % int(sc[1]), Color8(230, 140, 120), 12, 2)
			y += 10.0


func _matrix(ci: CanvasItem) -> void:
	var r := MATRIX
	Theme2.panel(ci, r)
	var output := campaign.reactor_output
	var draw := campaign.matrix_draw()
	var over := draw > output
	Theme2.label(ci, r.position + Vector2(6, 11), "ENERGY MATRIX", Theme2.AMBER, 8)
	Theme2.text(ci, Vector2(r.end.x - 6, r.position.y + 11), "%d/%du" % [int(draw), int(output)], Theme2.RED if over else Theme2.GREEN, 12, 2)
	_matrix_btns.clear()
	var y := r.position.y + 15.0
	for i in Rules.SYSTEMS:
		var on: bool = campaign.matrix[i]
		var cost: float = Rules.MATRIX_COST[i]
		var can_on := on or draw + cost <= output
		var col := Theme2.sys_color(i)
		var btn := Rect2(r.position.x + 6, y, 30, 10)
		_matrix_btns.append(Rect2(r.position.x + 2, y - 1, r.size.x - 4, 12))  # the whole row toggles
		Theme2.button(ci, btn, "WIRED" if on else "OFF", col, can_on, on)
		Theme2.label(ci, Vector2(r.position.x + 40, y + 8), Rules.SYS_NAME[i], col if on else Theme2.DIM, 8)
		Theme2.text(ci, Vector2(r.position.x + 70, y + 8), "%du" % int(cost), Theme2.TEXT if on else Theme2.DIM, 12)
		y += 11.0


func _cards(ci: CanvasItem) -> void:
	ci.draw_rect(CARDS.grow(2), Color(0, 0, 0, 0.6))
	Theme2.label(ci, Vector2(6, 121), "SALVAGE: CHOOSE ONE", Theme2.AMBER, 8)
	var n := maxi(campaign.salvage.size(), 1)
	var cw := (CARDS.size.x - CARD_GAP * (n - 1)) / n
	_install_btns.clear()
	for i in campaign.salvage.size():
		var id: int = campaign.salvage[i]
		var info: Array = Campaign.PARTS[id]
		var maker: int = info[1]
		var accent: Color = Campaign.MAKER_ACCENT[maker]
		var cr := Rect2(CARDS.position + Vector2(i * (cw + CARD_GAP), 0), Vector2(cw, CARDS.size.y))
		var is_chosen := chosen == id
		ci.draw_rect(cr, Theme2.PANEL)
		ci.draw_rect(cr, accent if is_chosen else Theme2.EDGE_HI, false, 2.0 if is_chosen else 1.0)
		ci.draw_rect(Rect2(cr.position, Vector2(cr.size.x, 10)), Color(accent, 0.22))
		Theme2.label(ci, cr.position + Vector2(3, 8), Campaign.MAKER_NAME[maker].split(" ")[0].split("-")[0], accent, 8)
		Theme2.label(ci, Vector2(cr.end.x - 3, cr.position.y + 8), Rules.SECTION_SHORT[info[2]], Theme2.DIM, 8, 2)
		# Icon on its plinth
		var ir := Rect2(cr.position + Vector2(3, 13), Vector2(24, 24))
		ci.draw_rect(ir, Theme2.PANEL_DARK)
		ci.draw_rect(Rect2(ir.position.x + 3, ir.end.y - 3, ir.size.x - 6, 2), Color8(70, 66, 60))
		ci.draw_rect(Rect2(ir.position.x + 3, ir.end.y - 3, ir.size.x - 6, 1), Color8(120, 114, 100))
		var bob := roundf(sin(t * 2.0 + i) * 1.0)
		Pixel.draw(ci, Sprites.ICONS[info[6]], ir.position + Vector2(2, 3 + bob))
		Theme2.text_wrap(ci, Vector2(cr.position.x + 30, cr.position.y + 22), info[0], cw - 33, Color.WHITE, 12, 2)
		var y := cr.position.y + 50.0
		y += Theme2.text_wrap(ci, Vector2(cr.position.x + 3, y), "+ " + info[3], cw - 6, Theme2.GREEN, 12, 2) + 2.0
		y += Theme2.text_wrap(ci, Vector2(cr.position.x + 3, y), "- " + info[4], cw - 6, Color8(240, 120, 100), 12, 2)
		if maker != Campaign.Maker.BASTION:
			Theme2.text(ci, Vector2(cr.position.x + 3, cr.end.y - 19), "-11% ORIGINAL", Theme2.AMBER, 12)
		var btn := Rect2(cr.position.x + 3, cr.end.y - 16, cw - 6, 14)
		_install_btns.append(btn)
		Theme2.button(ci, btn, "INSTALLED" if is_chosen else "INSTALL", accent, chosen < 0, is_chosen)


func tap(p: Vector2) -> void:
	for i in _matrix_btns.size():
		if _matrix_btns[i].has_point(p):
			var on: bool = campaign.matrix[i]
			if on or campaign.matrix_draw() + Rules.MATRIX_COST[i] <= campaign.reactor_output:
				campaign.matrix[i] = not on
			return
	if chosen < 0:
		for i in _install_btns.size():
			if _install_btns[i].has_point(p):
				chosen = campaign.salvage[i]
				campaign.install(chosen)
				return
	if DEPLOY_BTN.has_point(p):
		request = "deploy"


func key(keycode: int) -> void:
	if keycode == KEY_ENTER or keycode == KEY_SPACE:
		request = "deploy"
