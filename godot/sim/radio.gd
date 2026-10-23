## The radio log: who said what, when. Short lines, newest last.
class_name Radio
extends RefCounted

## Each entry: {time, voice, text}
var log: Array[Dictionary] = []
var time := 0.0


func say(voice: int, text: String) -> void:
	log.append({"time": time, "voice": voice, "text": text})
	if log.size() > 60:
		log.pop_front()


func last() -> Dictionary:
	return log.back() if not log.is_empty() else {}
