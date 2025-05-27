@tool
extends Control

const MIN_CELLS: int = 6
const MAX_CELLS: int = 15

@export_range(MIN_CELLS, MAX_CELLS, 1) var columns: int = MIN_CELLS:
	set(value):
		if !grid_container:
			return
		value = clampi(value, MIN_CELLS, MAX_CELLS)
		columns = value
		grid_container.columns = columns
		_fill_grid_with_cells(columns * columns)

@export var grid_container: GridContainer

@onready var camera_2d: Camera2D = $Camera2D

var cell_scene: PackedScene = preload("res://developer/cell.tscn")

func _ready() -> void:
	_fill_grid_with_cells(columns*columns)

func _fill_grid_with_cells(amount: int):
	_empty_grid()
	for i in range(amount):
		var cell = cell_scene.instantiate() as Cell
		grid_container.add_child(cell)
		cell.owner = grid_container
		if !cell.gui_input.is_connected(_on_gui_input):
			cell.gui_input.connect(_on_gui_input)

func _empty_grid():
	for c in grid_container.get_children():
		c.queue_free()


func _on_gui_input(event: InputEvent) -> void:
	if event is InputEventScreenDrag:
		camera_2d.position -= event.relative
	
	if event is InputEventMagnifyGesture:
		var zoom_x = clampf(camera_2d.zoom.x * event.factor, 0.6, 2.4)
		var zoom_y = clampf(camera_2d.zoom.y * event.factor, 0.6, 2.4)
		camera_2d.zoom.x = zoom_x
		camera_2d.zoom.y = zoom_y
