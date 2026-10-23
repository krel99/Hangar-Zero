## Seeded RNG so encounters (and screenshot scenes) are reproducible, plus the stateless
## hash noise the procedural art uses.
class_name Rng
extends RefCounted

var _r := RandomNumberGenerator.new()


func _init(seed_value: int = 1987) -> void:
	_r.seed = seed_value


func f() -> float:
	return _r.randf()


func range_f(lo: float, hi: float) -> float:
	return _r.randf_range(lo, hi)


func chance(p: float) -> bool:
	return _r.randf() < p


func index(n: int) -> int:
	return _r.randi() % maxi(n, 1)


static func hash2(x: int, y: int, seed_value: int) -> int:
	var h := ((x & 0xFFFFFFFF) * 374761393) ^ ((y & 0xFFFFFFFF) * 668265263) ^ ((seed_value & 0xFFFFFFFF) * 2246822519)
	h &= 0xFFFFFFFF
	h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
	return (h ^ (h >> 16)) & 0xFFFFFFFF


static func hashf(x: int, y: int, seed_value: int) -> float:
	return float(hash2(x, y, seed_value) & 0xFFFF) / 65535.0
