## Bakes every sprite and backdrop headlessly and times it; writes the backdrops to the
## user:// folder so they can be looked at.
##   godot --headless --path godot -s tests/test_art.gd
extends SceneTree


func _init() -> void:
	var t0 := Time.get_ticks_msec()
	for rows in [Sprites.SCOUT, Sprites.ARTILLERY, Sprites.CARRIER, Sprites.FACE_PILOT, Sprites.ARM_RAIL]:
		var tex := Pixel.texture(rows)
		assert(tex.get_width() > 0)
	for g in Sprites.ICONS + Sprites.HOTBAR:
		Pixel.texture(g)
	print("sprites: %d ms" % (Time.get_ticks_msec() - t0))
	t0 = Time.get_ticks_msec()
	var far := City.city_far()
	print("city far: %d ms" % (Time.get_ticks_msec() - t0))
	t0 = Time.get_ticks_msec()
	var near := City.city_near()
	print("city near: %d ms" % (Time.get_ticks_msec() - t0))
	t0 = Time.get_ticks_msec()
	var hangar := City.hangar_backdrop()
	print("hangar: %d ms" % (Time.get_ticks_msec() - t0))
	var out := OS.get_environment("HZ_ART_OUT")
	if out != "":
		far.save_png(out + "/far.png")
		near.save_png(out + "/near.png")
		hangar.save_png(out + "/hangar_bg.png")
		print("wrote backdrops to ", out)
	print("hull map ", Hull.map.size(), " detail ", Hull.detail.size(), " at(19,20)=", Hull.at(19, 20))
	print("ALL PASSED")
	quit()
