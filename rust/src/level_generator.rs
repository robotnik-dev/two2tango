use std::collections::HashMap;

use gdrust_kit::utils::fuzzy::{FuzzyRule, FuzzySet, FuzzySystem};
use godot::{classes::Engine, prelude::*};

use crate::{
    grid_cell::{Border, CellProps, ConstraintDirection, ConstraintProps, Symbol},
    level::{Level, MAX_COLUMNS, MIN_COLUMNS},
    level_manager::LevelState,
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

    pub fn applier(&self, level: Gd<Level>, settings: LevelSettings) -> SettingsApplier {
        SettingsApplier::new(level, settings)
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DifficultyLevel {
    Easy,
    #[default]
    Normal,
    Hard,
    Nightmare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelParameter {
    Columns,
    Constraints,
    VisibleSymbols,
}

#[derive(Debug, Default)]
pub struct LevelSettings {
    pub columns: i32,
    pub constraints: i32,
    pub visible_symbols: i32,
}

#[derive(Debug, Clone)]
pub struct LevelDifficultySystem {
    fuzzy_system: FuzzySystem<DifficultyLevel, LevelParameter>,
}

impl Default for LevelDifficultySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelDifficultySystem {
    pub fn new() -> Self {
        let mut system = FuzzySystem::new();

        system.add_input_set(FuzzySet::new(
            DifficultyLevel::Easy,
            vec![(0.0, 1.0), (4.0, 0.0)],
        ));

        system.add_input_set(FuzzySet::new(
            DifficultyLevel::Normal,
            vec![(0.0, 0.0), (4.0, 1.0), (6.0, 0.0)],
        ));

        system.add_input_set(FuzzySet::new(
            DifficultyLevel::Hard,
            vec![(4.0, 0.0), (6.0, 1.0), (10.0, 0.0)],
        ));

        system.add_input_set(FuzzySet::new(
            DifficultyLevel::Nightmare,
            vec![(6.0, 0.0), (10.0, 1.0)],
        ));

        // Define rules
        system.add_rule(
            FuzzyRule::new(DifficultyLevel::Easy)
                .with_consequence(LevelParameter::Columns, 0.0)
                .with_consequence(LevelParameter::Constraints, 1.0)
                .with_consequence(LevelParameter::VisibleSymbols, 1.0),
        );

        system.add_rule(
            FuzzyRule::new(DifficultyLevel::Normal)
                .with_consequence(LevelParameter::Columns, 0.33)
                .with_consequence(LevelParameter::Constraints, 0.66)
                .with_consequence(LevelParameter::VisibleSymbols, 0.66),
        );

        system.add_rule(
            FuzzyRule::new(DifficultyLevel::Hard)
                .with_consequence(LevelParameter::Columns, 0.66)
                .with_consequence(LevelParameter::Constraints, 0.33)
                .with_consequence(LevelParameter::VisibleSymbols, 0.33),
        );

        system.add_rule(
            FuzzyRule::new(DifficultyLevel::Nightmare)
                .with_consequence(LevelParameter::Columns, 1.0)
                .with_consequence(LevelParameter::Constraints, 0.0)
                .with_consequence(LevelParameter::VisibleSymbols, 0.0),
        );

        Self {
            fuzzy_system: system,
        }
    }

    pub fn get_settings(&self, difficulty_level: f32) -> HashMap<LevelParameter, f32> {
        self.fuzzy_system.evaluate(difficulty_level)
    }

    pub fn apply_settings(&self, difficulty_level: f32) -> LevelSettings {
        let settings = self.get_settings(difficulty_level);

        const MAX_CONSTRAINTS_PER_ROW_COL: i32 = 2;

        let columns = *settings.get(&LevelParameter::Columns).unwrap_or(&0.0);
        let visible_symbols = *settings
            .get(&LevelParameter::VisibleSymbols)
            .unwrap_or(&0.0);
        let constraints = *settings.get(&LevelParameter::Constraints).unwrap_or(&0.0);

        let mut actual_columns = MIN_COLUMNS as f32 + (MAX_COLUMNS - MIN_COLUMNS) as f32 * columns;
        // round up to an even number
        if actual_columns as i32 % 2 != 0 {
            actual_columns = actual_columns.ceil()
        }
        let max_visible_symbols = actual_columns * actual_columns * 0.25; // never more than this perentage should be visible, even at lowest difficulty
        let min_visible_symbols = 0.05 * (max_visible_symbols);
        let actual_visible_symbols =
            min_visible_symbols + (max_visible_symbols - min_visible_symbols) * visible_symbols;
        let min_constraints = max_visible_symbols * 0.05;
        let actual_constraints = min_constraints
            + actual_columns * MAX_CONSTRAINTS_PER_ROW_COL as f32 * 2.0 * constraints;

        LevelSettings {
            columns: actual_columns as i32,
            visible_symbols: actual_visible_symbols as i32,
            constraints: actual_constraints as i32,
        }
    }
}

#[derive(GodotClass, Default)]
#[class(init)]
pub struct SettingsApplier {
    level: Option<Gd<Level>>,
    settings: LevelSettings,
}

impl SettingsApplier {
    pub fn new(level: Gd<Level>, settings: LevelSettings) -> SettingsApplier {
        Self {
            level: Some(level),
            settings,
        }
    }

    /// Applies the seetings to the level except the 'columns' because the LevelBuilder already did that
    pub fn apply(&self) -> Gd<Level> {
        let level = self.level.as_ref().unwrap().clone();
        // TODO
        for _ in 0..self.settings.constraints {
            let maybe_ids_and_symbol = self.get_two_in_row(level.clone());
        }

        // --- Make Symbols hidden again except for a number of symbols specified in the settings
        // let amount_visible_symbols = self.settings.visible_symbols;
        // let amount_hidden_symbols = level.bind().get_ids().len() as i32 - amount_visible_symbols;
        // let mut ids = level.bind().get_ids().clone();
        // ids.shuffle();
        // for (_, id) in (0..amount_hidden_symbols).zip(ids.iter_shared()) {
        //     level
        //         .bind()
        //         .get_cell(id)
        //         .unwrap()
        //         .bind_mut()
        //         .switch_to(Symbol::None);
        // }
        level
    }

    /// Check if grid has any two-in-a-row cells and return the ids
    fn get_two_in_row(&self, level: Gd<Level>) -> Option<((i32, i32), Symbol)> {
        let grid_size = level.bind().get_columns();

        // Check rows
        for row in 0..grid_size {
            for col in 0..=(grid_size - 2) {
                let id1 = self.coords_to_id(row, col, grid_size);
                let id2 = self.coords_to_id(row, col + 1, grid_size);

                let (s1, s2) = (
                    level.bind().get_cell(id1).unwrap().bind().get_symbol(),
                    level.bind().get_cell(id2).unwrap().bind().get_symbol(),
                );
                if s1 != Symbol::None && s1 == s2 {
                    return Some(((id1, id2), s1));
                }
            }
        }

        // Check columns
        for col in 0..grid_size {
            for row in 0..=(grid_size - 2) {
                let id1 = self.coords_to_id(row, col, grid_size);
                let id2 = self.coords_to_id(row + 1, col, grid_size);

                let (s1, s2) = (
                    level.bind().get_cell(id1).unwrap().bind().get_symbol(),
                    level.bind().get_cell(id2).unwrap().bind().get_symbol(),
                );
                if s1 != Symbol::None && s1 == s2 {
                    return Some(((id1, id2), s1));
                }
            }
        }

        None
    }

    fn coords_to_id(&self, row: i32, col: i32, grid_size: i32) -> i32 {
        row * grid_size + col
    }
}

#[derive(GodotClass, Default)]
#[class(init)]
pub struct LevelBuilder {
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

    pub fn generate_level_from_state(&mut self, state: &LevelState, grid_size: i32) -> Gd<Level> {
        let level_scene = load::<PackedScene>(LEVEL_SCENE_PATH);
        let mut level = level_scene.instantiate_as::<Level>();
        let mut cell_props = vec![];
        for id in 0..grid_size * grid_size {
            let symbol = state.cells.get(&id).unwrap_or(&Symbol::None).clone();
            let prop = CellProps {
                id,
                symbol,
                constraint_props: vec![],
            };
            cell_props.push(prop);
        }
        level.bind_mut().build(cell_props);
        level
    }

    pub fn columns(&mut self, columns: i32) -> Result<&mut Self, String> {
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
        &mut self,
        id: i32,
        constraint_props: ConstraintProps,
    ) -> Result<&mut Self, String> {
        unimplemented!()
        // check if id is out of bounds
        // if id >= self.columns * self.columns || id < 0 {
        //     return Err(format!(
        //         "Id: {id} is out of bounds. Size of the grid: {}",
        //         self.columns * self.columns
        //     ));
        // }

        // // check if this exact constraint already exists
        // if self.constraints.iter().any(|p| {
        //     p.constraint_props.iter().any(|c| {
        //         c.constraint == constraint_props.constraint
        //             && c.constraint_direction == constraint_props.constraint_direction
        //     } && p.id == id)
        // }) {
        //     return Err(format!(
        //         "Constraint: {constraint_props:?} for id: {id} already exist"
        //     ));
        // }

        // // check if the cell prop already exists. remove the existing one and edit it
        // let cell_prop = match self.constraints.iter().position(|p| p.id == id) {
        //     Some(index) => {
        //         let cell_props = self.constraints.swap_remove(index);
        //         let mut con_props = cell_props.constraint_props.clone();
        //         con_props.push(constraint_props.clone());
        //         CellProps {
        //             id,
        //             constraint_props: con_props,
        //             ..Default::default()
        //         }
        //     }
        //     None => CellProps {
        //         id,
        //         constraint_props: vec![constraint_props.clone()],
        //         ..Default::default()
        //     },
        // };
        // self.constraints.push(cell_prop.clone());

        // // visit only the contraint that just got added
        // // for c_props in cell_prop.constraint_props.iter() {
        // let (pair_id, direction) = match constraint_props.constraint_direction {
        //     ConstraintDirection::Top => {
        //         if self.get_border(id) == Border::Top {
        //             return Err(format!(
        //                     "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the top most row!",
        //                     constraint_props.constraint,
        //                     constraint_props.constraint_direction,
        //                     id
        //                 ));
        //         };
        //         (id - self.columns, ConstraintDirection::Bot)
        //     }
        //     ConstraintDirection::Right => {
        //         if self.get_border(id) == Border::Right {
        //             return Err(format!(
        //                     "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the right most row!",
        //                     constraint_props.constraint,
        //                     constraint_props.constraint_direction,
        //                     id
        //                 ));
        //         };
        //         (id + 1, ConstraintDirection::Left)
        //     }
        //     ConstraintDirection::Bot => {
        //         if self.get_border(id) == Border::Bot {
        //             return Err(format!(
        //                     "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the bottom most row!",
        //                     constraint_props.constraint,
        //                     constraint_props.constraint_direction,
        //                     id
        //                 ));
        //         };
        //         (id + self.columns, ConstraintDirection::Top)
        //     }
        //     ConstraintDirection::Left => {
        //         if self.get_border(id) == Border::Left {
        //             return Err(format!(
        //                     "Cant set constraint: {:?} for direction: {:?}, because id: {} is in the left most row!",
        //                     constraint_props.constraint,
        //                     constraint_props.constraint_direction,
        //                     id
        //                 ));
        //         };
        //         (id - 1, ConstraintDirection::Right)
        //     }
        //     ConstraintDirection::None => {
        //         return Err("Setting constraint 'None' doesnt do anything".into())
        //     }
        // };

        // // like above: check if the cell prop for the pair id already exists. remove the existing one and edit it
        // let pair_cell_prop = match self.constraints.iter().position(|p| p.id == pair_id) {
        //     Some(index) => {
        //         let existing_cell_props = self.constraints.swap_remove(index);
        //         let mut con_props_list = existing_cell_props.constraint_props.clone();
        //         let new_constraint_props = ConstraintProps {
        //             constraint: constraint_props.constraint.clone(),
        //             constraint_direction: direction.clone(),
        //         };
        //         // push only when the new constraint doesnt already exist
        //         if !con_props_list.contains(&new_constraint_props) {
        //             con_props_list.push(new_constraint_props.clone());
        //         }
        //         CellProps {
        //             id,
        //             constraint_props: con_props_list,
        //             ..Default::default()
        //         }
        //     }
        //     None => CellProps {
        //         id: pair_id,
        //         constraint_props: vec![ConstraintProps {
        //             constraint: constraint_props.constraint.clone(),
        //             constraint_direction: direction.clone(),
        //         }],
        //         ..Default::default()
        //     },
        // };

        // self.constraints.push(pair_cell_prop);
        // Ok(self)
    }

    /// Consumes the builder and builds the level with given parameter
    pub fn build(self) -> Gd<Level> {
        let level_scene = load::<PackedScene>(LEVEL_SCENE_PATH);
        let mut level = level_scene.instantiate_as::<Level>();

        // join cell properties with the same id under just one property
        let mut final_constraints: HashMap<i32, CellProps> = HashMap::new();

        for cell_p in self.constraints {
            if let Some(entry) = final_constraints.get(&cell_p.id) {
                let mut new_entry = entry.clone();
                new_entry.constraint_props.extend(cell_p.constraint_props);
                final_constraints.insert(cell_p.id, new_entry);
            } else {
                final_constraints.insert(cell_p.id, cell_p);
            }
        }

        let mut cell_props = vec![];
        for id in 0..self.columns * self.columns {
            let props = match final_constraints.get(&id) {
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
}
