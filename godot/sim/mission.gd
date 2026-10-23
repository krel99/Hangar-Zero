## One operation: the clock, the wave table position, what was damaged, and the outcome.
class_name Mission
extends RefCounted

var number: int
var time := 0.0
var paused := false
var briefing := true
var outcome: int = Rules.Outcome.NONE
var next_wave := 0
var serial := 1
var damage_taken: Array[float] = []
var warned: Array[String] = []


func _init(n: int) -> void:
	number = n
	damage_taken.resize(Rules.SECTIONS)
	damage_taken.fill(0.0)


func running() -> bool:
	return not paused and not briefing and outcome == Rules.Outcome.NONE


func warn_once(key: String) -> bool:
	if warned.has(key):
		return false
	warned.append(key)
	return true
