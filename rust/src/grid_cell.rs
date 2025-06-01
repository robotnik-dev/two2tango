use godot::{
    classes::{
        control::MouseFilter, Container, Control, IControl, InputEvent, InputEventMouseButton,
        StyleBoxFlat, Texture2D, TextureRect,
    },
    global::MouseButtonMask,
    prelude::*,
};

const CONSTRAINT_TOP_IDX: i32 = 1;
const CONSTRAINT_RIGHT_IDX: i32 = 5;
const CONSTRAINT_BOT_IDX: i32 = 7;
const CONSTRAINT_LEFT_IDX: i32 = 3;

const PATH_TO_X_IMAGE: &str = "res://assets/symbols/moon.png";
const PATH_TO_Y_IMAGE: &str = "res://assets/symbols/sun.png";
const PATH_TO_EQUAL_IMAGE: &str = "res://assets/symbols/moon.png";
const PATH_TO_NON_EQUAL_IMAGE: &str = "res://assets/symbols/sun.png";
const PATH_TO_INVALID_IMAGE: &str = "res://assets/symbols/invalid.png";

const STYLEBOX_PATH_NOFOCUS: &str = "res://resources/grid_cell_nofocus.tres";
const STYLEBOX_PATH_FOCUS: &str = "res://resources/grid_cell_focus.tres";

#[derive(GodotConvert, Var, Export, Default, Debug, Clone, PartialEq)]
#[godot(via=GString)]
pub enum Constraint {
    #[default]
    None,
    Equal,
    NonEqual,
}

#[derive(GodotConvert, Var, Export, Default, Debug, Clone, PartialEq)]
#[godot(via=GString)]
pub enum Symbol {
    #[default]
    None,
    X,
    Y,
}

#[derive(GodotClass)]
#[class(tool, init, base=Control)]
pub struct GridCell {
    #[export]
    #[var(get = get_symbol, set = set_symbol)]
    pub symbol: Symbol,

    #[export]
    #[var(get, set = set_invalid)]
    pub invalid: bool,

    #[export]
    #[var(get = get_constraint_top, set = set_constraint_top)]
    pub constraint_top: Constraint,

    #[export]
    #[var(get = get_constraint_right, set = set_constraint_right)]
    pub constraint_right: Constraint,

    #[export]
    #[var(get = get_constraint_bot, set = set_constraint_bot)]
    pub constraint_bot: Constraint,

    #[export]
    #[var(get = get_constraint_left, set = set_constraint_left)]
    pub constraint_left: Constraint,

    #[export]
    symbol_container: OnEditor<Gd<Container>>,

    #[export]
    pub constraint_container: OnEditor<Gd<Container>>,

    #[export]
    pub invalid_symbol_container: OnEditor<Gd<Container>>,

    #[export]
    pub panel_container: OnEditor<Gd<Container>>,

    pub id: i32,

    base: Base<Control>,
}

#[godot_api]
impl IControl for GridCell {
    fn gui_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(mouse_event) = event.try_cast::<InputEventMouseButton>() {
            if mouse_event.is_pressed() && mouse_event.get_button_mask() == MouseButtonMask::LEFT {
                self.switch_to_next_symbol();
                let id = self.id;
                self.signals().clicked().emit(id);
            }
        }
    }
}

#[godot_api]
impl GridCell {
    #[signal]
    pub fn clicked(id: i32);

    #[signal]
    pub fn invalid_changed(invalid: bool);

    #[signal]
    pub fn symbol_changed(symbol: GString);

    #[signal]
    pub fn constraint_top_changed(constraint: GString);

    #[signal]
    pub fn constraint_right_changed(constraint: GString);

    #[signal]
    pub fn constraint_bot_changed(constraint: GString);

    #[signal]
    pub fn constraint_left_changed(constraint: GString);

    pub fn set_invalid_helper(&mut self, invalid: bool) {
        self.set_invalid(invalid);
    }

    pub fn set_disabled(&mut self, disabled: bool, focus: bool) {
        if disabled {
            self.base_mut().set_mouse_filter(MouseFilter::IGNORE);
        } else {
            self.base_mut().set_mouse_filter(MouseFilter::STOP);
        }
        if focus {
            self.panel_container
                .add_theme_stylebox_override("panel", &load::<StyleBoxFlat>(STYLEBOX_PATH_FOCUS));
        } else {
            self.panel_container
                .add_theme_stylebox_override("panel", &load::<StyleBoxFlat>(STYLEBOX_PATH_NOFOCUS));
        }
    }

    #[func]
    fn set_invalid(&mut self, invalid: bool) {
        self.invalid = invalid;

        self.clear_invalid_symbol_container();

        if invalid {
            let texture = load::<Texture2D>(PATH_TO_INVALID_IMAGE);
            let mut texture_rect = TextureRect::new_alloc();
            texture_rect.set_texture(&texture);
            self.invalid_symbol_container.add_child(&texture_rect);
            texture_rect.set_owner(&self.to_gd());
        }

        self.signals().invalid_changed().emit(invalid.to_godot());
    }

    #[func]
    fn set_symbol(&mut self, symbol: Symbol) {
        self.symbol = symbol.clone();
        let maybe_path = match symbol {
            Symbol::None => None,
            Symbol::X => Some(PATH_TO_X_IMAGE),
            Symbol::Y => Some(PATH_TO_Y_IMAGE),
        };

        self.clear_symbol_container();

        if let Some(path) = maybe_path {
            let texture = load::<Texture2D>(path);
            let mut texture_rect = TextureRect::new_alloc();
            texture_rect.set_texture(&texture);
            self.symbol_container.add_child(&texture_rect);
            texture_rect.set_owner(&self.to_gd());
        }

        self.signals().symbol_changed().emit(&symbol.to_godot());
    }

    #[func]
    pub fn get_symbol(&mut self) -> Symbol {
        self.symbol.clone()
    }

    #[func]
    fn set_constraint_top(&mut self, constraint: Constraint) {
        self.constraint_top = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };

        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_TOP_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                    }
                    None => texture_rect.set_texture(Gd::null_arg()),
                };
                self.signals()
                    .constraint_top_changed()
                    .emit(&constraint.to_godot());
            }
        }
    }

    #[func]
    pub fn get_constraint_top(&self) -> Constraint {
        self.constraint_top.clone()
    }

    #[func]
    fn set_constraint_right(&mut self, constraint: Constraint) {
        self.constraint_right = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };
        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_RIGHT_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                    }
                    None => texture_rect.set_texture(Gd::null_arg()),
                };
                self.signals()
                    .constraint_right_changed()
                    .emit(&constraint.to_godot());
            }
        }
    }

    #[func]
    pub fn get_constraint_right(&self) -> Constraint {
        self.constraint_right.clone()
    }

    #[func]
    fn set_constraint_bot(&mut self, constraint: Constraint) {
        self.constraint_bot = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };
        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_BOT_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                    }
                    None => texture_rect.set_texture(Gd::null_arg()),
                };
                self.signals()
                    .constraint_bot_changed()
                    .emit(&constraint.to_godot());
            }
        }
    }

    #[func]
    pub fn get_constraint_bot(&self) -> Constraint {
        self.constraint_bot.clone()
    }

    #[func]
    fn set_constraint_left(&mut self, constraint: Constraint) {
        self.constraint_left = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_IMAGE),
        };
        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_LEFT_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                    }
                    None => texture_rect.set_texture(Gd::null_arg()),
                };
                self.signals()
                    .constraint_left_changed()
                    .emit(&constraint.to_godot());
            }
        }
    }

    #[func]
    pub fn get_constraint_left(&self) -> Constraint {
        self.constraint_left.clone()
    }

    fn switch_to_next_symbol(&mut self) {
        let new_symbol = match self.symbol {
            Symbol::None => Symbol::X,
            Symbol::X => Symbol::Y,
            Symbol::Y => Symbol::None,
        };
        self.set_symbol(new_symbol);
    }

    fn clear_symbol_container(&mut self) {
        for mut c in self.symbol_container.get_children().iter_shared() {
            c.queue_free();
        }
    }

    fn clear_invalid_symbol_container(&mut self) {
        for mut c in self.invalid_symbol_container.get_children().iter_shared() {
            c.queue_free();
        }
    }
}
