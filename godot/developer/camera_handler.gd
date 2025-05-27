extends Node2D

@export_range(0.0, 1.0, 0.1) var camera_speed = 0.1

enum DragState {
	Idle,
	Dragging,
}

var drag: DragState = DragState.Idle
var drag_start_mouse_pos: Vector2

@onready var camera_2d: Camera2D = $Camera2D

func _input(event: InputEvent) -> void:
	pass
	#if event is InputEventMouseButton:
		#if event.button_mask == MOUSE_BUTTON_LEFT:
			#if drag != DragState.Dragging:
				#drag_start_mouse_pos = get_viewport().get_mouse_position()
			#drag = DragState.Dragging
		#else:
			#drag = DragState.Idle
	#
	#if drag == DragState.Dragging:
		#if event is InputEventMouseMotion:
			#var direction = drag_start_mouse_pos.direction_to(get_viewport().get_mouse_position())
			#camera_2d.move_local_x(direction.x)
			#camera_2d.move_local_y(direction.y)
