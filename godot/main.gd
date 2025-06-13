extends Node

@export var level_manager: LevelManager
@export var camera: Camera2D
@export var back_button: Button

var UILevelGenerationScene: PackedScene = preload("res://ui/ui_level_generation.tscn")

func _ready() -> void:
	open_level_generation_scene()

func open_level_generation_scene():
	var ui_level_generation = UILevelGenerationScene.instantiate() as UILevelGeneration
	add_child(ui_level_generation)
	ui_level_generation.level_selected_to_play.connect(_on_level_selected)

func _on_level_selected(level: Level):
	for c in get_children():
		if c is UILevelGeneration:
			c.queue_free()
	
	level_manager.start_level(level)
	camera.make_current()
	back_button.show()
	camera.position = level.position


func _on_ui_back_button_pressed() -> void:
	level_manager.quit_level()
	back_button.hide()
	open_level_generation_scene()
