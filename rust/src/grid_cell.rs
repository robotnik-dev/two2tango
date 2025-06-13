use godot::{
    classes::{
        control::MouseFilter, texture_rect::ExpandMode, Container, Control, IControl, InputEvent,
        InputEventMouseButton, StyleBoxFlat, Texture2D, TextureRect,
    },
    global::{randf, MouseButtonMask},
    prelude::*,
};

const CONSTRAINT_TOP_IDX: i32 = 1;
const CONSTRAINT_RIGHT_IDX: i32 = 5;
const CONSTRAINT_BOT_IDX: i32 = 7;
const CONSTRAINT_LEFT_IDX: i32 = 3;

const PATH_TO_X_IMAGE: &str = "res://assets/symbols/symbols_X.svg";
const PATH_TO_Y_IMAGE: &str = "res://assets/symbols/symbols_Y.svg";
const PATH_TO_EQUAL_A_IMAGE: &str = "res://assets/symbols/symbols_Equal_A.svg";
const PATH_TO_EQUAL_B_IMAGE: &str = "res://assets/symbols/symbols_Equal_B.svg";
const PATH_TO_NON_EQUAL_A_IMAGE: &str = "res://assets/symbols/symbols_NonEqual_A.svg";
const PATH_TO_NON_EQUAL_B_IMAGE: &str = "res://assets/symbols/symbols_NonEqual_B.svg";
const PATH_TO_INVALID_IMAGE: &str = "res://assets/symbols/symbols_invalid.svg";

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

impl Constraint {
    pub fn random_not_none() -> Constraint {
        if randf() > 0.5 {
            Constraint::Equal
        } else {
            Constraint::NonEqual
        }
    }
}

#[derive(GodotConvert, Var, Export, Default, Debug, Clone, PartialEq)]
#[godot(via=GString)]
pub enum Symbol {
    #[default]
    None,
    X,
    Y,
}

#[derive(GodotConvert, Var, Export, Default, Debug, Clone, PartialEq)]
#[godot(via=GString)]
pub enum Border {
    #[default]
    None,
    Top,
    Right,
    Bot,
    Left,
}

#[derive(GodotClass)]
#[class(no_init)]
pub struct CellProps {
    pub id: i32,
    pub symbol: Symbol,
    pub constraint_props: Array<Gd<ConstraintProps>>,
}

#[godot_api]
impl CellProps {
    pub fn new(id: i32, symbol: Symbol, constraint_props: Array<Gd<ConstraintProps>>) -> Gd<Self> {
        Gd::from_object(Self {
            id,
            symbol,
            constraint_props,
        })
    }
}

#[derive(GodotClass)]
#[class(init)]
pub struct ConstraintProps {
    pub constraint: Constraint,
    pub constraint_direction: ConstraintDirection,
}

#[godot_api]
impl ConstraintProps {
    pub fn new(constraint: Constraint, constraint_direction: ConstraintDirection) -> Gd<Self> {
        Gd::from_object(Self {
            constraint,
            constraint_direction,
        })
    }
}

#[derive(GodotConvert, Var, Export, Debug, Clone, PartialEq, Default)]
#[godot(via=GString)]
pub enum ConstraintDirection {
    #[default]
    None,
    Top,
    Right,
    Bot,
    Left,
}

impl ConstraintDirection {
    pub fn random_not_none() -> ConstraintDirection {
        let r = randf();
        if r < 0.25 {
            ConstraintDirection::Top
        } else if r < 0.5 {
            ConstraintDirection::Right
        } else if r < 7.5 {
            ConstraintDirection::Bot
        } else {
            ConstraintDirection::Left
        }
    }
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
    symbol_container: OnEditor<Gd<Container>>,

    #[export]
    pub constraint_container: OnEditor<Gd<Container>>,

    #[export]
    pub invalid_symbol_container: OnEditor<Gd<Container>>,

    #[export]
    pub panel_container: OnEditor<Gd<Container>>,

    #[var]
    pub id: i32,

    #[init(val = CellProps::new(0, Symbol::None, array![]))]
    pub props: Gd<CellProps>,

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

    pub fn setup(&mut self, props: Gd<CellProps>) {
        self.props = props;
        self.set_id(self.props.clone().bind().id);
        self.set_symbol(self.props.clone().bind().symbol.clone());
        let props = self.props.bind().constraint_props.clone();
        for prop in props.iter_shared() {
            self.set_constraint(prop.clone());
        }
    }

    pub fn get_constraint_props(&self) -> Array<Gd<ConstraintProps>> {
        self.props.bind().constraint_props.clone()
    }

    pub fn set_invalid_rust(&mut self, invalid: bool) {
        self.set_invalid(invalid);
    }

    pub fn add_constraint_props(&mut self, constraint_props: Gd<ConstraintProps>) {
        self.props
            .bind_mut()
            .constraint_props
            .push(&constraint_props);
        let props = self.props.bind().constraint_props.clone();
        for prop in props.iter_shared() {
            self.set_constraint(prop.clone());
        }
    }

    pub fn set_constraint(&mut self, constraint_props: Gd<ConstraintProps>) {
        match constraint_props.bind().constraint_direction {
            ConstraintDirection::Top => {
                self.set_constraint_top(constraint_props.clone().bind().constraint.clone())
            }
            ConstraintDirection::Right => {
                self.set_constraint_right(constraint_props.clone().bind().constraint.clone())
            }
            ConstraintDirection::Bot => {
                self.set_constraint_bot(constraint_props.clone().bind().constraint.clone())
            }
            ConstraintDirection::Left => {
                self.set_constraint_left(constraint_props.clone().bind().constraint.clone())
            }
            ConstraintDirection::None => {}
        }
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
            texture_rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
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
            texture_rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
            self.symbol_container.add_child(&texture_rect);
            texture_rect.set_owner(&self.to_gd());
        }

        self.signals().symbol_changed().emit(&symbol.to_godot());
    }

    #[func]
    pub fn get_symbol(&self) -> Symbol {
        self.symbol.clone()
    }

    #[func]
    fn set_constraint_top(&mut self, constraint: Constraint) {
        self.constraint_top = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_A_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_A_IMAGE),
        };

        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_TOP_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                        texture_rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
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
    fn set_constraint_right(&mut self, constraint: Constraint) {
        self.constraint_right = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_A_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_A_IMAGE),
        };
        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_RIGHT_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                        texture_rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
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
    fn set_constraint_bot(&mut self, constraint: Constraint) {
        self.constraint_bot = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_B_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_B_IMAGE),
        };
        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_BOT_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                        texture_rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
                        // B variant needs to be on top of A so changing draw order here
                        texture_rect.set_z_index(20);
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
    fn set_constraint_left(&mut self, constraint: Constraint) {
        self.constraint_left = constraint.clone();
        let maybe_path = match constraint {
            Constraint::None => None,
            Constraint::Equal => Some(PATH_TO_EQUAL_B_IMAGE),
            Constraint::NonEqual => Some(PATH_TO_NON_EQUAL_B_IMAGE),
        };
        if let Some(c) = self.constraint_container.get_child(CONSTRAINT_LEFT_IDX) {
            if let Ok(mut texture_rect) = c.try_cast::<TextureRect>() {
                match maybe_path {
                    Some(path) => {
                        let texture = load::<Texture2D>(path);
                        texture_rect.set_texture(&texture);
                        texture_rect.set_expand_mode(ExpandMode::IGNORE_SIZE);
                        // B variant needs to be on top of A so changing draw order here
                        texture_rect.set_z_index(20);
                    }
                    None => texture_rect.set_texture(Gd::null_arg()),
                };
                self.signals()
                    .constraint_left_changed()
                    .emit(&constraint.to_godot());
            }
        }
    }

    pub fn switch_to(&mut self, symbol: Symbol) {
        self.set_symbol(symbol);
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
