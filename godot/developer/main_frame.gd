extends Control

@export var camera_2d: Camera2D

func _input(event):
	if event is InputEventScreenDrag:
		camera_2d.position -= event.relative
		get_viewport().set_input_as_handled()
	
	if event is InputEventMagnifyGesture:
		var zoom_x = clampf(camera_2d.zoom.x * event.factor, 0.6, 2.4)
		var zoom_y = clampf(camera_2d.zoom.y * event.factor, 0.6, 2.4)
		camera_2d.zoom.x = zoom_x
		camera_2d.zoom.y = zoom_y
		get_viewport().set_input_as_handled()
	
