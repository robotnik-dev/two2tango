use godot::prelude::*;
use level_generator::LevelGenerator;

mod entrypoint;
mod grid_cell;
mod level;
mod level_generator;

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {
    fn on_level_init(level: InitLevel) {
        LevelGenerator::register(level);
    }

    fn on_level_deinit(level: InitLevel) {
        LevelGenerator::unregister(level);
    }
}
