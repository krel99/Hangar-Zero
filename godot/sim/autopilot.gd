## A crude scripted pilot used by the tests and the `play` tool: snipes what it can see,
## vents when hot, scrams on overload, leaves the chair when the hull is crawling.
class_name Autopilot
extends RefCounted


static func act(sim: Sim) -> void:
	var mech := sim.mech
	var best: Entities.Enemy = null
	var best_rank := -1
	for e in sim.enemies:
		if e.dist > mech.st.radar_range:
			continue
		var rank: int = [1, 3, 2][e.kind]
		if rank > best_rank:
			best_rank = rank
			best = e
	mech.target = best.id if best else -1
	# Look where the target is, if that camera still works.
	if best:
		var cam := Rules.camera_for(best.angle)
		if mech.camera_ok(cam):
			mech.view = Rules.camera_bearing(cam)
	if mech.heat < 70.0 and mech.rail_charge >= 1.0:
		sim.cmd_fire_rail = true
	if mech.heat > 88.0:
		sim.cmd_vent = true
	if mech.overload >= 0.0:
		sim.cmd_scram = true
	var worst := 0
	for s in range(1, 9):
		if mech.frac(s) < mech.frac(worst):
			worst = s
	mech.repair_target = worst if mech.frac(worst) < 0.3 else Rules.Section.UPLINK
	if sim.boarders.size() >= 3 and mech.in_seat():
		sim.cmd_crawl_to = sim.boarders[0].section
	if mech.pilot == Rules.PilotState.WORKING:
		var still_here := false
		for b in sim.boarders:
			if b.section == mech.pilot_at:
				still_here = true
		if not still_here:
			sim.cmd_return = true
