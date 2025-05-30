use godot::{
    classes::{
        Container, Control, IControl, InputEvent, InputEventMouseButton, Texture2D, TextureRect,
    },
    global::MouseButtonMask,
    prelude::*,
};

const CONSTRAINT_TOP_IDX: i32 = 1;
const CONSTRAINT_RIGHT_IDX: i32 = 5;
const CONSTRAINT_BOT_IDX: i32 = 7;
const CONSTRAINT_LEFT_IDX: i32 = 3;

const PATH_TO_MOON_IMAGE: &str = "res://assets/symbols/moon.png";
const PATH_TO_SUN_IMAGE: &str = "res://assets/symbols/sun.png";
const PATH_TO_EQUAL_IMAGE: &str = "res://assets/symbols/moon.png";
const PATH_TO_NON_EQUAL_IMAGE: &str = "res://assets/symbols/sun.png";

#[derive(GodotConvert, Var, Export, Default, Debug, Clone)]
#[godot(via=GString)]
pub enum Constraint {
    #[default]
    None,
    Equal,
    NonEqual,
}

#[derive(GodotConvert, Var, Export, Default, Debug, Clone)]
#[godot(via=GString)]
pub enum Symbol {
    #[default]
    None,
    Moon,
    Sun,
}

#[derive(GodotClass)]
#[class(tool, init, base=Control)]
pub struct GridCell {
    #[export]
    #[var(get, set = set_symbol)]
    pub symbol: Symbol,

    #[export]
    #[var(get, set = set_constraint_top)]
    pub constraint_top: Constraint,

    #[export]
    #[var(get, set = set_constraint_right)]
    pub constraint_right: Constraint,

    #[export]
    #[var(get, set = set_constraint_bot)]
    pub constraint_bot: Constraint,

    #[export]
    #[var(get, set = set_constraint_left)]
    pub constraint_left: Constraint,

    #[export]
    symbol_container: Option<Gd<Container>>,

    #[export]
    pub constraint_container: Option<Gd<Container>>,

    pub id: i32,

    base: Base<Control>,
}

#[godot_api]
impl IControl for GridCell {
    fn ready(&mut self) {}

    fn gui_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
            if mouse_event.is_pressed() && mouse_event.get_button_mask() == MouseButtonMask::LEFT {
                let id = self.id;
                self.base_mut().emit_signal("clicked", &[id.to_variant()]);
                self.switch_to_next_symbol();
            }
        }
    }
}

#[godot_api]
impl GridCell {
    #[signal]
    fn clicked(id: i32);

    #[signal]
    fn constraint_top_changed(constraint: Constraint);

    #[signal]
    fn constraint_right_changed(constraint: Constraint);

    #[signal]
    fn constraint_bot_changed(constraint: Constraint);

    #[signal]
    fn constraint_left_changed(constraint: Constraint);

    fn switch_to_next_symbol(&mut self) {
        let new_symbol = match self.symbol {
            Symbol::None => Symbol::Moon,
            Symbol::Moon => Symbol::Sun,
            Symbol::Sun => Symbol::None,
        };
        self.set_symbol(new_symbol);
    }

    fn clear_symbol_container(&mut self) {
        if let Some(container) = self.get_symbol_container() {
            for mut c in container.get_children().iter_shared() {
                c.queue_free();
            }
        }
    }

    #[func]
    fn set_symbol(&mut self, symbol: Symbol) {
        self.symbol = symbol.clone();
        let maybe_path = match symbol {
            Symbol::None => None,
            Symbol::Moon => Some(PATH_TO_MOON_IMAGE),
            Symbol::Sun => Some(PATH_TO_SUN_IMAGE),
        };

        self.clear_symbol_container();
        if let Some(mut container) = self.get_symbol_container() {
            if let Some(path) = maybe_path {
                let texture = load::<Texture2D>(path);
                let mut texture_rect = TextureRect::new_alloc();
                texture_rect.set_texture(&texture);
                container.add_child(&texture_rect);
                texture_rect.set_owner(&self.to_gd());
            }
        }
    }

    #[func]
    fn set_constraint_top(&mut self, constraint: Constraint) {
        self.constraint_top = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };
        if let Some(container) = self.get_constraint_container() {
            if let Some(c) = container.get_child(CONSTRAINT_TOP_IDX) {
                if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                    match maybe_path {
                        Some(path) => {
                            let texture = load::<Texture2D>(path);
                            texture_rect.set_texture(&texture);
                        }
                        None => texture_rect.set_texture(Gd::null_arg()),
                    };
                    self.base_mut()
                        .emit_signal("constraint_top_changed", &[constraint.to_variant()]);
                }
            }
        }
    }
    #[func]
    fn set_constraint_right(&mut self, constraint: Constraint) {
        self.constraint_right = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };
        if let Some(container) = self.get_constraint_container() {
            if let Some(c) = container.get_child(CONSTRAINT_RIGHT_IDX) {
                if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                    match maybe_path {
                        Some(path) => {
                            let texture = load::<Texture2D>(path);
                            texture_rect.set_texture(&texture);
                        }
                        None => texture_rect.set_texture(Gd::null_arg()),
                    };
                    self.base_mut()
                        .emit_signal("constraint_right_changed", &[constraint.to_variant()]);
                }
            }
        }
    }
    #[func]
    fn set_constraint_bot(&mut self, constraint: Constraint) {
        self.constraint_bot = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };
        if let Some(container) = self.get_constraint_container() {
            if let Some(c) = container.get_child(CONSTRAINT_BOT_IDX) {
                if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                    match maybe_path {
                        Some(path) => {
                            let texture = load::<Texture2D>(path);
                            texture_rect.set_texture(&texture);
                        }
                        None => texture_rect.set_texture(Gd::null_arg()),
                    };
                    self.base_mut()
                        .emit_signal("constraint_bot_changed", &[constraint.to_variant()]);
                }
            }
        }
    }
    #[func]
    fn set_constraint_left(&mut self, constraint: Constraint) {
        self.constraint_left = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };
        if let Some(container) = self.get_constraint_container() {
            if let Some(c) = container.get_child(CONSTRAINT_LEFT_IDX) {
                if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                    match maybe_path {
                        Some(path) => {
                            let texture = load::<Texture2D>(path);
                            texture_rect.set_texture(&texture);
                        }
                        None => texture_rect.set_texture(Gd::null_arg()),
                    };
                    self.base_mut()
                        .emit_signal("constraint_left_changed", &[constraint.to_variant()]);
                }
            }
        }
    }
}
