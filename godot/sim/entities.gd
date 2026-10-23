## The things in the world besides the mech: enemies, boarding drones (in flight and
## latched), artillery shells, and short-lived visual effects.
class_name Entities
extends RefCounted


class Enemy:
	var id: int
	var kind: int
	var serial: int
	var hp: float
	var max_hp: float
	var angle: float
	var dist: float
	var hold_dist: float
	var state: int = Rules.EState.APPROACH
	var timer: float
	var orbit: float
	var stun := 0.0
	var drones := 4
	var hit_flash := 0.0
	var reveal := 0.0
	## Where the radar thinks it is (exact when a camera covers it).
	var est_angle: float
	var est_dist: float
	## Bearing error of the estimate in radians; 0 when camera-covered.
	var err := 0.0
	var covered := false
	## Mission time of the last radar sample.
	var last_sweep := -10.0

	## Hit-chance multiplier for a shot aimed at the radar estimate.
	func aim_factor() -> float:
		return 1.0 if covered else clampf(1.0 - rad_to_deg(err) / 12.0, 0.25, 1.0)

	func callsign() -> String:
		return "%s-%02d" % [Rules.KIND_CALLSIGN[kind], serial]


class Drone:
	var angle: float
	var dist: float
	var hp := 14.0


class Boarder:
	var section: int
	var age := 0.0


class Shell:
	var target: int
	var t := 0.0
	var flight := 4.5
	var angle: float


class Fx:
	var kind: int
	var angle: float
	var dist: float
	var t := 0.0
	var ttl: float
