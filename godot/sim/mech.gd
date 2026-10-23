## The walking fortress: ten sections with their own integrity, the reactor bus, heat,
## shields, cameras and where the pilot is.
class_name Mech
extends RefCounted

var hp: Array[float] = []
var max_hp: Array[float] = []
var alloc: Array[float] = []
var heat := 22.0
var heat_peak := 22.0
var shield := 30.0
var shield_hit_t := 0.0
var rail_charge := 0.4
var rail_auto := false
var weapons_free := true
var point_defence := true
## Enemy id (see Sim.enemies), or -1.
var target := -1
var repair_target: int = Rules.Section.UPLINK
var vent_t := 0.0
var vent_cd := 0.0
var shields_down_t := 0.0
var scram_t := 0.0
var scram_cd := 0.0
var ecm_cd := 0.0
## Seconds left before containment fails, or -1 when not overloading.
var overload := -1.0
## Seconds left before the interlocks fail, or -1 when the cockpit is sealed.
var breach := -1.0
var interlocks_failed := false
var pilot: int = Rules.PilotState.SEAT
var pilot_at: int = Rules.Section.COCKPIT
var pilot_t := 0.0
var pilot_dur := 1.0
var sweep := 0.0
var ac_firing := false
var ac_burst_t := 0.0
var hit_flash := 0.0
var shake := 0.0
var kills := 0
var cameras: Array[float] = []
## Seconds-ish since each section last took damage (1 = just now), for the diagram.
var hit_t: Array[float] = []
var cams_at_start := 0
## Continuous view bearing (the phone turns it); cameras are the arcs that still see.
var view := 0.0
var quad_view := false

## Derived numbers, refreshed every step (see Sim.derive).
var st := {}


func _init(campaign: Campaign) -> void:
	hp.assign(campaign.max_hp)
	hp[Rules.Section.UPLINK] = 0.0
	max_hp.assign(campaign.max_hp)
	alloc = campaign.alloc()
	cameras.assign(campaign.cameras)
	hit_t.resize(Rules.SECTIONS)
	hit_t.fill(0.0)
	for c in cameras:
		if c >= Rules.CAMERA_MIN:
			cams_at_start += 1
	st = Sim.derive(self, campaign, 0)


func frac(s: int) -> float:
	return clampf(hp[s] / maxf(max_hp[s], 1.0), 0.0, 1.0)


## How well a section works: destroyed sections stop, damaged ones limp along.
func eff(s: int) -> float:
	return 0.0 if hp[s] <= 0.0 else 0.3 + 0.7 * frac(s)


func camera_ok(cam: int) -> bool:
	return cameras[cam] >= Rules.CAMERA_MIN and hp[Rules.Section.SENSORS] > 0.0


## True when a working camera is looking at this bearing.
func covers(angle: float) -> bool:
	return camera_ok(Rules.camera_for(angle))


func cameras_up() -> int:
	var n := 0
	for c in Rules.CAMERAS:
		if camera_ok(c):
			n += 1
	return n


func uplink() -> float:
	return hp[Rules.Section.UPLINK]


func in_seat() -> bool:
	return pilot == Rules.PilotState.SEAT


func damage(s: int, amount: float) -> void:
	hp[s] = maxf(hp[s] - amount, 0.0)
	if amount > 0.3:
		hit_t[s] = 1.0
