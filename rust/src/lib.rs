use godot::{
    classes::{ISprite2D, Sprite2D},
    prelude::*,
};

struct GodotRustExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotRustExtension {}

// TODO: remove below when using the template
#[derive(GodotClass)]
#[class(init, base=Sprite2D)]
struct Example {
    base: Base<Sprite2D>,
}

#[godot_api]
impl ISprite2D for Example {
    fn process(&mut self, delta: f64) {
        self.base_mut().rotate(1.0 * delta as f32);
    }
}
