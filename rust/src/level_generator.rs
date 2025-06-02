use godot::{classes::Engine, prelude::*};

use crate::{
    grid_cell::{Border, CellProps, ConstraintDirection, ConstraintProps},
    level::{Level, MAX_COLUMNS, MIN_COLUMNS},
};

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
    constraints: Vec<CellProps>,
}

#[godot_api]
impl LevelBuilder {
    pub fn new() -> LevelBuilder {
        LevelBuilder {
            columns: MIN_COLUMNS,
            ..Default::default()
        }
    }

    pub fn difficulty(mut self, difficulty: Difficulty) -> LevelBuilder {
        self.difficulty = difficulty;
        self
    }

    pub fn columns(mut self, columns: i32) -> Result<LevelBuilder, String> {
        if columns % 2 != 0 {
            return Err(format!("Columns '{columns}' must be an even number"));
        };
        if columns < MIN_COLUMNS {
            return Err(format!(
                "Columns '{columns}' must be more than {MIN_COLUMNS}"
            ));
        };
        if columns > MAX_COLUMNS {
            return Err(format!(
                "Columns '{columns}' must be less than {MAX_COLUMNS}"
            ));
        };

        self.columns = columns;
        Ok(self)
    }

    /// set the given constraint with the id and also with the corresponding id the constraint points to
    pub fn set_constraint(
        mut self,
        id: i32,
        constraint_props: ConstraintProps,
    ) -> Result<LevelBuilder, String> {
        // check if id is out of bounds
        if id >= self.columns * self.columns || id < 0 {
            return Err(format!(
                "Id: {id} is out of bounds. Size of the grid: {}",
                self.columns * self.columns
            ));
        }

        let cell_prop = CellProps {
            id,
            constraint_props: constraint_props.clone(),
            ..Default::default()
        };

        let (pair_id, direction) = match constraint_props.constraint_direction {
            ConstraintDirection::Top => {
                if self.get_border(id) == Border::Top {
                    return Err(format!(
                        "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the top most row!",
                        constraint_props.constraint,
                        constraint_props.constraint_direction,
                        id
                    ));
                };
                (id - self.columns, ConstraintDirection::Bot)
            }
            ConstraintDirection::Right => {
                if self.get_border(id) == Border::Right {
                    return Err(format!(
                        "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the right most row!",
                        constraint_props.constraint,
                        constraint_props.constraint_direction,
                        id
                    ));
                };
                (id + 1, ConstraintDirection::Left)
            }
            ConstraintDirection::Bot => {
                if self.get_border(id) == Border::Bot {
                    return Err(format!(
                        "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the bottom most row!",
                        constraint_props.constraint,
                        constraint_props.constraint_direction,
                        id
                    ));
                };
                (id + self.columns, ConstraintDirection::Top)
            }
            ConstraintDirection::Left => {
                if self.get_border(id) == Border::Left {
                    return Err(format!(
                        "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the left most row!",
                        constraint_props.constraint,
                        constraint_props.constraint_direction,
                        id
                    ));
                };
                (id - 1, ConstraintDirection::Right)
            }
            ConstraintDirection::None => {
                return Err("Setting constraint 'None' doesnt do anything".into())
            }
        };

        let pair_cell_prop = CellProps {
            id: pair_id,
            constraint_props: ConstraintProps {
                constraint: constraint_props.constraint,
                constraint_direction: direction,
            },
            ..Default::default()
        };
        self.constraints.push(cell_prop);
        self.constraints.push(pair_cell_prop);
        Ok(self)
    }

    /// Consumes the builder and builds the level with given parameter
    pub fn build(self) -> Gd<Level> {
        let level_scene = load::<PackedScene>(LEVEL_SCENE_PATH);
        let mut level = level_scene.instantiate_as::<Level>();

        let mut cell_props = vec![];
        for id in 0..self.columns * self.columns {
            let props = match self.constraints.iter().find(|&prop| prop.id == id) {
                // already setup properties, just use this one
                Some(props) => props.clone(),
                // generate default properties
                None => CellProps {
                    id,
                    ..Default::default()
                },
            };
            cell_props.push(props);
        }
        level.bind_mut().build(cell_props);
        level
    }

    /// Get the indices of the row and the column of a cell starting with (row: 0, col: 0) in the top left
    fn get_row_col(&self, id: i32) -> (i32, i32) {
        let row = id / self.columns;
        let col = id % self.columns;
        (row, col)
    }

    /// Get the border type of the cell id
    fn get_border(&self, id: i32) -> Border {
        let (r, c) = self.get_row_col(id);
        let border_top = r == 0;
        let border_bot = r == self.columns - 1;
        let border_left = c == 0;
        let border_right = c == self.columns - 1;
        if border_top {
            Border::Top
        } else if border_bot {
            Border::Bot
        } else if border_left {
            Border::Left
        } else if border_right {
            Border::Right
        } else {
            Border::None
        }
    }
}
