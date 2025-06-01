use godot::{classes::Engine, prelude::*};

use crate::level::{Level, MIN_CELLS};

pub const SINGLETON_NAME: &str = "LevelGenerator";
pub const LEVEL_SCENE_PATH: &str = "res://level/level.tscn";

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct LevelGenerator {
    base: Base<Object>,
}

#[godot_api]
impl LevelGenerator {
    pub fn builder(&self) -> LevelBuilder {
        LevelBuilder::new()
    }

    /// Used to register the singleton inside the ExtensionLibrary crate once for the main game library during the
    /// `InitLevel::Scene` phase
    pub fn register(level: InitLevel) {
        if level == InitLevel::Scene {
            Engine::singleton().register_singleton(SINGLETON_NAME, &LevelGenerator::new_alloc());
        }
    }

    /// Used to unregister the singleton inside the ExtensionLibrary crate once for the main game library during the
    /// `InitLevel::Scene` phase
    pub fn unregister(level: InitLevel) {
        if level == InitLevel::Scene {
            let mut engine = Engine::singleton();

            if let Some(singleton) = engine.get_singleton(SINGLETON_NAME) {
                engine.unregister_singleton(SINGLETON_NAME);
                singleton.free();
            } else {
                godot_error!("Failed to get singleton: {SINGLETON_NAME}");
            }
        }
    }
}

#[derive(Debug, Default)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

#[derive(GodotClass, Default)]
#[class(init)]
pub struct LevelBuilder {
    difficulty: Difficulty,
    columns: i32,
}

#[godot_api]
impl LevelBuilder {
    pub fn new() -> LevelBuilder {
        LevelBuilder {
            columns: MIN_CELLS,
            ..Default::default()
        }
    }

    pub fn difficulty(mut self, difficulty: Difficulty) -> LevelBuilder {
        self.difficulty = difficulty;
        self
    }

    pub fn columns(mut self, columns: i32) -> LevelBuilder {
        self.columns = columns;
        self
    }

    #[must_use]
    /// Consumes the builder and builds the level with given parameter
    pub fn build(self) -> Gd<Level> {
        let level_scene = load::<PackedScene>(LEVEL_SCENE_PATH);
        let mut level = level_scene.instantiate_as::<Level>();
        level.bind_mut().set_columns(self.columns);
        level
    }
}
