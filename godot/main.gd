extends Node

@export var level_manager: LevelManager
@export var camera: Camera2D

var UILevelGenerationScene: PackedScene = preload("res://ui/ui_level_generation.tscn")


func _ready() -> void:
	var ui_level_generation = UILevelGenerationScene.instantiate() as UILevelGeneration
	add_child(ui_level_generation)
	ui_level_generation.level_selected_to_play.connect(_on_level_selected)


func _on_level_selected(level: Level):
	for c in get_children():
		if c is UILevelGeneration:
			c.queue_free()
	
	level_manager.start_level(level)
	camera.make_current()
