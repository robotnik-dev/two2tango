use godot::prelude::*;

mod grid_cell;
mod level;
mod level_builder;
mod level_manager;

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}
