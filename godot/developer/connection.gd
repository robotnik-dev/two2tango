extends Resource
class_name Connection

enum ConnectionType {
	Equal,
	NotEqual,
}

@export var type: ConnectionType
