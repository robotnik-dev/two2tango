extends Control
class_name UILevelGeneration

signal level_selected_to_play(level: Level)

@export var UIDifficultySlider: HSlider
@export var UILevelPreview: SubViewport
@export var UIDifficultyPreviewLabel: Label
@export var camera: Camera2D
@export var progress_bar: ProgressBar
@export var error_label: Label
@export var UIPlayButton: Button

@onready var builder: LevelBuilder = LevelBuilder.new()

var thread: Thread
var current_preview: Level

func _ready() -> void:
	builder.made_progress.connect(_on_progress)
	builder.built.connect(_on_built)
	builder.timeout.connect(_on_timeout)

func _remove_preview() -> void:
	for c in UILevelPreview.get_children():
		if c is Level:
			c.queue_free()


func _on_ui_generate_level_preview_button_pressed() -> void:
	if thread:
		if thread.is_alive():
			push_warning("LevelGeneration: Thread is already running")
			return
	
	UIPlayButton.hide()
	error_label.hide()
	progress_bar.value = 0.0
	if thread:
		thread.wait_to_finish()
	thread = Thread.new()
	thread.start(builder.build.bind(UIDifficultySlider.value))


func _on_ui_difficulty_slider_value_changed(value: float) -> void:
	UIDifficultyPreviewLabel.text = builder.difficulty_to_name(value)
	UIDifficultyPreviewLabel.label_settings.font_color = builder.difficulty_to_color(value)


func _on_built(level: Level) -> void:
	_remove_preview()
	var zoom = builder.columns_to_zoom(level.columns)
	camera.zoom = zoom
	UILevelPreview.call_deferred("add_child", level)
	UIPlayButton.show()
	current_preview = level


func _on_timeout() -> void:
	_remove_preview()
	error_label.show()


func _on_progress(value: float) -> void:
	progress_bar.value = clampf(value, progress_bar.value, value)


func _exit_tree() -> void:
	thread.wait_to_finish()


func _on_ui_play_button_pressed() -> void:
	if current_preview:
		level_selected_to_play.emit(current_preview)
