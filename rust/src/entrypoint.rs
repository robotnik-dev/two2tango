use godot::{
    classes::{Engine, SubViewport},
    prelude::*,
};

use crate::level_generator::{self, LevelGenerator};

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct Main {
    #[export]
    pub level_viewport: Option<Gd<SubViewport>>,

    base: Base<Node>,
}

#[godot_api]
impl INode for Main {
    fn ready(&mut self) {
        if let Some(mut viewport) = self.get_level_viewport() {
            if let Some(level_generator) =
                Engine::singleton().get_singleton(level_generator::SINGLETON_NAME)
            {
                let mut level = level_generator
                    .cast::<LevelGenerator>()
                    .bind_mut()
                    .builder()
                    .columns(8)
                    .difficulty(level_generator::Difficulty::Normal)
                    .build();
                viewport
                    .add_child_ex(&level)
                    .force_readable_name(true)
                    .done();
                level.set_owner(&self.to_gd());
            }
        }
    }
}

#[godot_api]
impl Main {}
