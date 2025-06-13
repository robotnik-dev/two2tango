use std::collections::HashMap;

use gdrust_kit::utils::fuzzy::{FuzzyRule, FuzzySet, FuzzySystem};
use godot::{classes::Time, global::randomize, prelude::*};

use crate::{
    grid_cell::{CellProps, Constraint, ConstraintDirection, ConstraintProps, Symbol},
    level::{Level, MAX_COLUMNS, MIN_COLUMNS},
};

pub const LEVEL_SCENE_PATH: &str = "res://level/level.tscn";
pub const TIMEOUT: f32 = 8.0; // in seconds

#[derive(Debug, Clone)]
pub struct LevelState {
    pub cells: HashMap<i32, Symbol>,
}

impl LevelState {
    pub fn progress(&self) -> f32 {
        self.cells.values().filter(|&s| *s != Symbol::None).count() as f32 / self.cells.len() as f32
    }
}

pub enum RowColumn {
    Row(i32),
    Col(i32),
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

    pub fn get_name_color_by(&self, difficulty: f32) -> (GString, Color) {
        const MAX_VALUE: f32 = 1.0;
        const EASY_THS: f32 = 2.0;
        const NORMAL_THS: f32 = 5.0;
        const HARD_THS: f32 = 8.0;
        const NIGHTMARE_THS: f32 = 10.0;
        let (name, color) = {
            if difficulty < EASY_THS {
                let color_value = difficulty / EASY_THS;
                let color = Color::from_rgb(
                    MAX_VALUE * color_value,
                    MAX_VALUE - MAX_VALUE * color_value,
                    0.0,
                );
                ("Easy", color)
            } else if difficulty < NORMAL_THS {
                let color_value = (difficulty - EASY_THS) / (NORMAL_THS - EASY_THS);
                let color = Color::from_rgb(
                    MAX_VALUE * color_value,
                    MAX_VALUE - MAX_VALUE * color_value,
                    0.0,
                );

                ("Normal", color)
            } else if difficulty < HARD_THS {
                let color_value = (difficulty - NORMAL_THS) / (HARD_THS - NORMAL_THS);
                let color = Color::from_rgb(
                    MAX_VALUE * color_value,
                    MAX_VALUE - MAX_VALUE * color_value,
                    0.0,
                );

                ("Hard", color)
            } else {
                let color_value = (difficulty - HARD_THS) / (NIGHTMARE_THS - HARD_THS);
                let color = Color::from_rgb(
                    MAX_VALUE * color_value,
                    MAX_VALUE - MAX_VALUE * color_value,
                    0.0,
                );

                ("Nightmare", color)
            }
        };
        (name.to_godot(), color)
    }

    pub fn get_settings(&self, difficulty_level: f32) -> HashMap<LevelParameter, f32> {
        self.fuzzy_system.evaluate(difficulty_level)
    }

    pub fn apply_settings(&self, difficulty: f32) -> LevelSettings {
        let settings = self.get_settings(difficulty);

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
        let max_constraints = actual_columns * actual_columns * 0.66; // never more than this percentage should be visible, even at lowest difficulty
        let min_constraints = max_constraints * 0.33; // never less than this percentage should be visible, even at lowest difficulty
        let actual_constraints =
            min_constraints + actual_columns * MAX_CONSTRAINTS_PER_ROW_COL as f32 * constraints;

        let max_visible_symbols = actual_columns * actual_columns - actual_constraints; // at least the amount of constraints need to be hidden
        let min_visible_symbols = actual_columns * actual_columns * 0.10; // never less than this percentage should be visible, even at lowest difficulty
        let actual_visible_symbols =
            min_visible_symbols + (max_visible_symbols - min_visible_symbols) * visible_symbols;

        LevelSettings {
            columns: actual_columns as i32,
            visible_symbols: actual_visible_symbols as i32,
            constraints: actual_constraints as i32,
        }
    }
}

#[derive(GodotClass)]
#[class(init, base=Object)]
pub struct LevelBuilder {
    #[init(val = MIN_COLUMNS)]
    #[var]
    pub columns: i32,

    pub difficulty_system: LevelDifficultySystem,

    base: Base<Object>,
}

#[godot_api]
impl LevelBuilder {
    #[signal]
    pub fn built(level: Gd<Level>);

    #[signal]
    pub fn timeout();

    #[signal]
    /// A value between 0 and 1
    pub fn made_progress(progress: f32);

    #[func]
    pub fn build(&mut self, difficulty: f32) {
        self.difficulty_system = LevelDifficultySystem::new();
        let settings = self.difficulty_system.apply_settings(difficulty);
        self.set_columns(settings.columns);

        let level = self.build_default();

        if let Some(solved_level) = self.solve_level(level.clone(), TIMEOUT as u64) {
            self.apply_settings(solved_level.clone(), settings);
            self.signals().built().emit(&solved_level);
        } else {
            self.signals().timeout().emit();
        }
    }

    #[func]
    pub fn build_from_cell_props(&mut self, cell_props: Array<Gd<CellProps>>) -> Gd<Level> {
        let level_scene = load::<PackedScene>(LEVEL_SCENE_PATH);
        let mut level = level_scene.instantiate_as::<Level>();

        level.bind_mut().build_with_props(cell_props);
        level
    }

    #[func]
    /// Helper to convert level size to appropriate zoom level for the camera
    pub fn columns_to_zoom(&mut self, columns: i32) -> Vector2 {
        let zoom = Vector2::new(1.0, 1.0);
        let factor = match columns {
            6 => 1.0,
            8 => 0.8,
            10 => 0.6,
            _ => todo!(),
        };

        zoom * factor
    }

    #[func]
    pub fn difficulty_to_name(&mut self, difficulty: f32) -> GString {
        self.difficulty_system.get_name_color_by(difficulty).0
    }

    #[func]
    pub fn difficulty_to_color(&mut self, difficulty: f32) -> Color {
        self.difficulty_system.get_name_color_by(difficulty).1
    }

    /// We set all the constraints and symbols in this level
    pub fn apply_settings(&self, level: Gd<Level>, settings: LevelSettings) {
        let mut checked_pairs = vec![];
        let mut constraints_set = 0;

        while constraints_set < settings.constraints {
            if let Some(((id1, id2), symbol)) = self.get_random_pair(level.clone(), &checked_pairs)
            {
                let constraint_props_id1 = match symbol {
                    Symbol::None => {
                        // Non Equal constraint
                        ConstraintProps::new(
                            Constraint::NonEqual,
                            self.get_contraint_direction((id1, id2)),
                        )
                    }
                    _ => {
                        // Equal constraint
                        ConstraintProps::new(
                            Constraint::Equal,
                            self.get_contraint_direction((id1, id2)),
                        )
                    }
                };

                let constraint_props_id2 = match constraint_props_id1.bind().constraint_direction {
                    ConstraintDirection::Bot => {
                        if constraint_props_id1.bind().constraint == Constraint::Equal {
                            ConstraintProps::new(Constraint::Equal, ConstraintDirection::Top)
                        } else {
                            ConstraintProps::new(Constraint::NonEqual, ConstraintDirection::Top)
                        }
                    }
                    ConstraintDirection::Top => {
                        if constraint_props_id1.bind().constraint == Constraint::Equal {
                            ConstraintProps::new(Constraint::Equal, ConstraintDirection::Bot)
                        } else {
                            ConstraintProps::new(Constraint::NonEqual, ConstraintDirection::Bot)
                        }
                    }
                    ConstraintDirection::Right => {
                        if constraint_props_id1.bind().constraint == Constraint::Equal {
                            ConstraintProps::new(Constraint::Equal, ConstraintDirection::Left)
                        } else {
                            ConstraintProps::new(Constraint::NonEqual, ConstraintDirection::Left)
                        }
                    }
                    ConstraintDirection::Left => {
                        if constraint_props_id1.bind().constraint == Constraint::Equal {
                            ConstraintProps::new(Constraint::Equal, ConstraintDirection::Right)
                        } else {
                            ConstraintProps::new(Constraint::NonEqual, ConstraintDirection::Right)
                        }
                    }
                    _ => ConstraintProps::new(Constraint::None, ConstraintDirection::None),
                };

                level
                    .bind()
                    .get_cell(id1)
                    .unwrap()
                    .bind_mut()
                    .add_constraint_props(constraint_props_id1);
                level
                    .bind()
                    .get_cell(id2)
                    .unwrap()
                    .bind_mut()
                    .add_constraint_props(constraint_props_id2);

                checked_pairs.push((id1, id2));
                constraints_set += 1;
            };
        }

        // --- Make Symbols hidden again except for a number of symbols specified in the settings
        // 1. hide one of the pairs selected above
        let amount_visible_symbols = settings.visible_symbols;
        let amount_hidden_symbols = level.bind().get_ids().len() as i32 - amount_visible_symbols;
        let mut cells_hidden = 0;
        for (idx, pair) in checked_pairs.iter().enumerate() {
            if idx >= amount_hidden_symbols as usize {
                break;
            }
            level
                .bind()
                .get_cell(pair.0)
                .unwrap()
                .bind_mut()
                .switch_to(Symbol::None);
            cells_hidden += 1;
        }

        let mut ids = level.bind().get_ids().clone();
        ids.shuffle();
        let left_to_hide = amount_hidden_symbols - cells_hidden;
        let mut iter = ids
            .iter_shared()
            .take(left_to_hide as usize)
            .collect::<Vec<i32>>();
        while let Some(id) = iter.pop() {
            level
                .bind()
                .get_cell(id)
                .unwrap()
                .bind_mut()
                .switch_to(Symbol::None);
        }
    }

    /// Returns the direction form first id to second id
    fn get_contraint_direction(&self, ids: (i32, i32)) -> ConstraintDirection {
        let id1 = ids.0;
        let id2 = ids.1;
        if id2 - id1 == 1 {
            ConstraintDirection::Right
        } else if id2 - id1 == -1 {
            ConstraintDirection::Left
        } else if id2 - id1 == self.columns {
            ConstraintDirection::Bot
        } else {
            ConstraintDirection::Top
        }
    }

    /// Returns the given neighbour id back when the constraint can be upheld, else return None
    fn get_border_free(
        &self,
        id: i32,
        neighbour: i32,
        constraint_direction: ConstraintDirection,
    ) -> Option<i32> {
        let first_row_col = 0;
        let last_row_col = self.columns - 1;
        let row = id / self.columns;
        let col = id % self.columns;
        let forbidden = {
            // 1. check the border rows and col
            (row == first_row_col && constraint_direction == ConstraintDirection::Top) // top row
            || (row == last_row_col && constraint_direction == ConstraintDirection::Bot) // bot row
            || (col == first_row_col && constraint_direction == ConstraintDirection::Left) // left col
            || (col == last_row_col && constraint_direction == ConstraintDirection::Right) // right col

            // 2. check the corners
            // top left corner
            || (row == first_row_col && col == first_row_col) && (constraint_direction == ConstraintDirection::Top || constraint_direction == ConstraintDirection::Left)
             // top right corner
            || (row == first_row_col && col == last_row_col) && (constraint_direction == ConstraintDirection::Top || constraint_direction == ConstraintDirection::Right)
             // bot right corner
            || (row == last_row_col && col == last_row_col) && (constraint_direction == ConstraintDirection::Bot || constraint_direction == ConstraintDirection::Right)
            // bot left corner
            || (row == last_row_col && col == first_row_col) && (constraint_direction == ConstraintDirection::Bot || constraint_direction == ConstraintDirection::Left)
        };
        if forbidden {
            None
        } else {
            Some(neighbour)
        }
    }

    /// Gets two random ids that are next to each other and the symbol if they are equal
    fn get_random_pair(
        &self,
        level: Gd<Level>,
        checked_pairs: &[(i32, i32)],
    ) -> Option<((i32, i32), Symbol)> {
        let mut ids = level.bind().get_ids();
        ids.shuffle();
        let constraint = Constraint::random_not_none();

        for id in ids.iter_shared() {
            let id1 = id;
            let constraint_direction = ConstraintDirection::random_not_none();
            let id2 = match constraint_direction {
                ConstraintDirection::Top => {
                    self.get_border_free(id1, id1 - self.columns, constraint_direction)
                }
                ConstraintDirection::Right => {
                    self.get_border_free(id1, id1 + 1, constraint_direction)
                }
                ConstraintDirection::Bot => {
                    self.get_border_free(id1, id1 + self.columns, constraint_direction)
                }
                ConstraintDirection::Left => {
                    self.get_border_free(id1, id1 - 1, constraint_direction)
                }
                ConstraintDirection::None => None,
            }?;

            let s1 = level.bind().get_cell(id1).map(|c| c.bind().get_symbol())?;
            let s2 = level.bind().get_cell(id2).map(|c| c.bind().get_symbol())?;

            match constraint {
                Constraint::Equal => {
                    if s1 != Symbol::None && s1 == s2 && !checked_pairs.contains(&(id1, id2)) {
                        return Some(((id1, id2), s1));
                    }
                }
                Constraint::NonEqual => {
                    if s1 != Symbol::None && s1 != s2 && !checked_pairs.contains(&(id1, id2)) {
                        return Some(((id1, id2), Symbol::None));
                    }
                }
                Constraint::None => {}
            }
        }
        None
    }

    /// Solve the level while setting all the symbols. Should return a different level solved level all the time
    ///
    /// THIS TAKES A WHILE TO RUN.
    fn solve_level(&mut self, level: Gd<Level>, max_duration: u64) -> Option<Gd<Level>> {
        // randomize level creation
        randomize();

        let mut cells = HashMap::new();
        for c in level.bind().get_cells().iter_shared() {
            let id = c.bind().get_id();
            let symbol = c.bind().get_symbol();
            cells.insert(id, symbol);
        }

        let mut stack: Vec<(LevelState, Option<i32>, Vec<Symbol>)> =
            vec![(LevelState { cells }, None, vec![])];
        let start = Time::singleton().get_ticks_msec();
        while let Some((mut current_state, last_cell, mut remaining_options)) = stack.pop() {
            // check for timeout
            if Time::singleton().get_ticks_msec() - start > max_duration * 1000 {
                return None;
            }

            // Update the progress made
            self.signals()
                .made_progress()
                .emit(current_state.progress());

            // 1. PROPAGATE CONSTRAINTS
            self.propagate_all_constraints(&mut current_state);

            // 2. CHECK IF STUCK
            if self.in_impossible_state(&current_state) {
                continue;
            }
            // 3. CHECK IF DONE
            if self.solved(&current_state) {
                if self.validate_final_solution(&current_state) {
                    godot_print!("Valid solution found!");
                    return Some(self.generate_level_from_state(&current_state));
                } else {
                    godot_print!("Invalid solution detected: keep searching");
                    continue; // Keep searching
                }
            }

            // 4. MAKE SMART GUESS
            let (cell, symbol) = if !remaining_options.is_empty() {
                // Continue with previous cell's remaining options
                (last_cell, remaining_options.pop().unwrap())
            } else {
                // Pick new cell and get all its options
                let cell = self.choose_best_cell(&current_state);
                if cell.is_none() {
                    continue; // No valid cells left, backtrack
                }

                remaining_options = self.get_valid_symbols(&current_state, cell.unwrap()); // safe unwrap because point is unreachable for cell == None
                if remaining_options.is_empty() {
                    continue; // No valid symbols for this cell, backtrack
                }

                let symbol = remaining_options.pop().unwrap();
                (cell, symbol)
            };

            // Try this symbol
            let mut new_state = current_state.clone();
            new_state.cells.insert(cell.unwrap(), symbol); // safe unwrap because point is unreachable for cell == None

            // 5. SAVE BACKTRACK POINT
            if !remaining_options.is_empty() {
                stack.push((current_state, cell, remaining_options));
            }

            // Continue with this guess
            stack.push((new_state, None, vec![]));
        }

        Some(level)
    }

    fn validate_final_solution(&self, state: &LevelState) -> bool {
        let grid_size = self.get_grid_size();
        godot_print!("Grid size: {:?}", grid_size);
        // Check row counts
        for row in 0..grid_size {
            let x_count = self.count_symbol_in_row(state, row, &Symbol::X);
            let y_count = self.count_symbol_in_row(state, row, &Symbol::Y);
            if x_count != grid_size / 2 || y_count != grid_size / 2 {
                godot_print!(
                    "Row {} has {} X's and {} Y's (should be {})",
                    row,
                    x_count,
                    y_count,
                    grid_size / 2
                );
                return false;
            }
        }

        // Check column counts
        for col in 0..grid_size {
            let x_count = self.count_symbol_in_col(state, col, &Symbol::X);
            let y_count = self.count_symbol_in_col(state, col, &Symbol::Y);
            if x_count != grid_size / 2 || y_count != grid_size / 2 {
                godot_print!(
                    "Col {} has {} X's and {} Y's (should be {})",
                    col,
                    x_count,
                    y_count,
                    grid_size / 2
                );
                return false;
            }
        }

        // Check three-in-a-row
        if self.has_three_in_row(state) {
            godot_print!("Three-in-a-row violation found!");
            return false;
        }

        true
    }

    /// Find cells that MUST be a certain symbol
    ///
    /// - Look for patterns like: X X ?  → ? must be Y
    /// - Look for patterns like: X ? X  → ? must be Y
    /// - Check if row has 3 X's already → remaining cells must be Y
    /// - Keep doing this until no more forced moves found
    fn propagate_all_constraints(&mut self, state: &mut LevelState) {
        let mut changed = true;
        let grid_size = self.get_grid_size();

        while changed {
            changed = false;

            // Check all empty cells for forced moves
            for id in 0..(grid_size * grid_size) {
                if *state.cells.get(&id).unwrap_or(&Symbol::None) == Symbol::None {
                    if let Some(forced_symbol) = self.get_forced_symbol(state, id) {
                        state.cells.insert(id, forced_symbol);
                        changed = true;
                        self.signals().made_progress().emit(state.progress());
                    }
                }
            }
        }
    }

    /// Helper function to determine if a cell must be a specific symbol
    fn get_forced_symbol(&self, state: &LevelState, id: i32) -> Option<Symbol> {
        let grid_size = self.get_grid_size();
        let (row, col) = self.id_to_coords(id);
        let mut forbidden = Vec::new();

        // Check horizontal three-in-a-row patterns
        // Pattern: XX? (would be third)
        if col >= 2 {
            let left1 = self.coords_to_id(row, col - 1);
            let left2 = self.coords_to_id(row, col - 2);
            if let (Some(s1), Some(s2)) = (state.cells.get(&left1), state.cells.get(&left2)) {
                if *s1 != Symbol::None && *s1 == *s2 {
                    forbidden.push(s1.clone());
                }
            }
        }

        // Pattern: X?X (would be middle)
        if col >= 1 && col < grid_size - 1 {
            let left = self.coords_to_id(row, col - 1);
            let right = self.coords_to_id(row, col + 1);
            if let (Some(s1), Some(s2)) = (state.cells.get(&left), state.cells.get(&right)) {
                if *s1 != Symbol::None && *s1 == *s2 {
                    forbidden.push(s1.clone());
                }
            }
        }

        // Pattern: ?XX (would be first)
        if col <= grid_size - 3 {
            let right1 = self.coords_to_id(row, col + 1);
            let right2 = self.coords_to_id(row, col + 2);
            if let (Some(s1), Some(s2)) = (state.cells.get(&right1), state.cells.get(&right2)) {
                if *s1 != Symbol::None && *s1 == *s2 {
                    forbidden.push(s1.clone());
                }
            }
        }

        // Check vertical three-in-a-row patterns
        // Pattern: XX? (would be third)
        if row >= 2 {
            let up1 = self.coords_to_id(row - 1, col);
            let up2 = self.coords_to_id(row - 2, col);
            if let (Some(s1), Some(s2)) = (state.cells.get(&up1), state.cells.get(&up2)) {
                if *s1 != Symbol::None && *s1 == *s2 {
                    forbidden.push(s1.clone());
                }
            }
        }

        // Pattern: X?X (would be middle)
        if row >= 1 && row < grid_size - 1 {
            let up = self.coords_to_id(row - 1, col);
            let down = self.coords_to_id(row + 1, col);
            if let (Some(s1), Some(s2)) = (state.cells.get(&up), state.cells.get(&down)) {
                if *s1 != Symbol::None && *s1 == *s2 {
                    forbidden.push(s1.clone());
                }
            }
        }

        // Pattern: ?XX (would be first)
        if row <= grid_size - 3 {
            let down1 = self.coords_to_id(row + 1, col);
            let down2 = self.coords_to_id(row + 2, col);
            if let (Some(s1), Some(s2)) = (state.cells.get(&down1), state.cells.get(&down2)) {
                if *s1 != Symbol::None && *s1 == *s2 {
                    forbidden.push(s1.clone());
                }
            }
        }

        // Check row symbol count constraints
        let row_x_count = self.count_symbol_in_row(state, row, &Symbol::X);
        let row_y_count = self.count_symbol_in_row(state, row, &Symbol::Y);
        if row_x_count >= grid_size / 2 {
            forbidden.push(Symbol::X);
        }
        if row_y_count >= grid_size / 2 {
            forbidden.push(Symbol::Y);
        }

        // Check column symbol count constraints
        let col_x_count = self.count_symbol_in_col(state, col, &Symbol::X);
        let col_y_count = self.count_symbol_in_col(state, col, &Symbol::Y);
        if col_x_count >= grid_size / 2 {
            forbidden.push(Symbol::X);
        }
        if col_y_count >= grid_size / 2 {
            forbidden.push(Symbol::Y);
        }

        // Determine forced symbol
        let forbidden_x = forbidden.contains(&Symbol::X);
        let forbidden_y = forbidden.contains(&Symbol::Y);

        match (forbidden_x, forbidden_y) {
            (true, false) => Some(Symbol::Y),
            (false, true) => Some(Symbol::X),
            _ => None, // Either both forbidden (impossible) or neither (not forced)
        }
    }

    /// Detect impossible situations early
    ///
    /// Checks for:
    /// - Row has 4 X's (impossible in 6x6 grid)
    /// - Three X's in a row already exist
    /// - Empty cell has no valid options
    fn in_impossible_state(&self, state: &LevelState) -> bool {
        let grid_size = self.get_grid_size();

        // Check for too many symbols in any row/column
        for i in 0..grid_size {
            let row_x = self.count_symbol_in_row(state, i, &Symbol::X);
            let row_y = self.count_symbol_in_row(state, i, &Symbol::Y);
            let col_x = self.count_symbol_in_col(state, i, &Symbol::X);
            let col_y = self.count_symbol_in_col(state, i, &Symbol::Y);

            if row_x > grid_size / 2
                || row_y > grid_size / 2
                || col_x > grid_size / 2
                || col_y > grid_size / 2
            {
                return true;
            }
        }

        // Check for three-in-a-row violations
        if self.has_three_in_row(state) {
            return true;
        }

        // Check for empty cells with no valid options
        for id in 0..(grid_size * grid_size) {
            if *state.cells.get(&id).unwrap_or(&Symbol::None) == Symbol::None
                && self.get_valid_symbols(state, id).is_empty()
            {
                return true;
            }
        }

        false
    }

    /// Pick the smartest cell to fill next
    ///
    /// Priority order:
    /// 1. Cells with only 1 valid option (almost forced)
    /// 2. Cells in rows/columns closest to symbol limits
    /// 3. Cells with fewest options
    fn choose_best_cell(&self, state: &LevelState) -> Option<i32> {
        let grid_size = self.get_grid_size();
        let mut best_cell = None;
        let mut min_options = usize::MAX;
        let mut max_constraint_pressure = 0;

        // randomize the order in which is searched to get a slightly different result each time
        let mut ids = Array::from_iter(0..grid_size * grid_size);
        ids.shuffle();

        for id in ids.iter_shared() {
            if *state.cells.get(&id).unwrap_or(&Symbol::None) == Symbol::None {
                let valid_options = self.get_valid_symbols(state, id);
                let option_count = valid_options.len();

                if option_count == 0 {
                    continue; // Skip impossible cells
                }

                // Calculate constraint pressure (how close row/col are to limits)
                let (row, col) = self.id_to_coords(id);
                let row_pressure = self.calculate_constraint_pressure(state, row, true);
                let col_pressure = self.calculate_constraint_pressure(state, col, false);
                let total_pressure = row_pressure + col_pressure;

                // Priority 1: Cells with only 1 option
                if option_count == 1 {
                    return Some(id);
                }

                // Priority 2 & 3: Fewest options + highest constraint pressure
                if option_count < min_options
                    || (option_count == min_options && total_pressure > max_constraint_pressure)
                {
                    best_cell = Some(id);
                    min_options = option_count;
                    max_constraint_pressure = total_pressure;
                }
            }
        }

        best_cell
    }

    /// Calculate how close a row/column is to its symbol limits
    fn calculate_constraint_pressure(&self, state: &LevelState, index: i32, is_row: bool) -> i32 {
        let grid_size = self.get_grid_size();
        let max_per_line = grid_size / 2;

        let (x_count, y_count) = if is_row {
            (
                self.count_symbol_in_row(state, index, &Symbol::X),
                self.count_symbol_in_row(state, index, &Symbol::Y),
            )
        } else {
            (
                self.count_symbol_in_col(state, index, &Symbol::X),
                self.count_symbol_in_col(state, index, &Symbol::Y),
            )
        };

        // Higher pressure when closer to limits
        (max_per_line - x_count.min(max_per_line)) + (max_per_line - y_count.min(max_per_line))
    }

    /// Returns ['Symbol::X', 'Symbol::Y'] minus any that would violate constraints
    /// Checks three-in-a-row and count limits
    fn get_valid_symbols(&self, state: &LevelState, id: i32) -> Vec<Symbol> {
        let mut valid = Vec::new();

        for symbol in [Symbol::X, Symbol::Y] {
            if self.is_valid_placement(state, id, &symbol) {
                valid.push(symbol);
            }
        }

        valid
    }

    /// Check if placing a symbol at a position would violate constraints
    fn is_valid_placement(&self, state: &LevelState, id: i32, symbol: &Symbol) -> bool {
        let (row, col) = self.id_to_coords(id);
        let grid_size = self.get_grid_size();

        // Check three-in-a-row constraint
        if self.would_create_three_in_row(state, row, col, symbol) {
            return false;
        }

        // Check count constraints
        let row_count = self.count_symbol_in_row(state, row, symbol);
        let col_count = self.count_symbol_in_col(state, col, symbol);

        if row_count >= grid_size / 2 || col_count >= grid_size / 2 {
            return false;
        }

        true
    }

    /// Check if placing symbol would create three in a row
    fn would_create_three_in_row(
        &self,
        state: &LevelState,
        row: i32,
        col: i32,
        symbol: &Symbol,
    ) -> bool {
        let grid_size = self.get_grid_size();

        // Check horizontal
        // Pattern: XX? (symbol would be third)
        if col >= 2 {
            let left1 = self.coords_to_id(row, col - 1);
            let left2 = self.coords_to_id(row, col - 2);
            if let (Some(s1), Some(s2)) = (state.cells.get(&left1), state.cells.get(&left2)) {
                if s1 == symbol && s2 == symbol {
                    return true;
                }
            }
        }

        // Pattern: X?X (symbol would be middle)
        if col >= 1 && col < grid_size - 1 {
            let left = self.coords_to_id(row, col - 1);
            let right = self.coords_to_id(row, col + 1);
            if let (Some(s1), Some(s2)) = (state.cells.get(&left), state.cells.get(&right)) {
                if s1 == symbol && s2 == symbol {
                    return true;
                }
            }
        }

        // Pattern: ?XX (symbol would be first)
        if col <= grid_size - 3 {
            let right1 = self.coords_to_id(row, col + 1);
            let right2 = self.coords_to_id(row, col + 2);
            if let (Some(s1), Some(s2)) = (state.cells.get(&right1), state.cells.get(&right2)) {
                if s1 == symbol && s2 == symbol {
                    return true;
                }
            }
        }

        // Check vertical (same patterns)
        if row >= 2 {
            let up1 = self.coords_to_id(row - 1, col);
            let up2 = self.coords_to_id(row - 2, col);
            if let (Some(s1), Some(s2)) = (state.cells.get(&up1), state.cells.get(&up2)) {
                if s1 == symbol && s2 == symbol {
                    return true;
                }
            }
        }

        if row >= 1 && row < grid_size - 1 {
            let up = self.coords_to_id(row - 1, col);
            let down = self.coords_to_id(row + 1, col);
            if let (Some(s1), Some(s2)) = (state.cells.get(&up), state.cells.get(&down)) {
                if s1 == symbol && s2 == symbol {
                    return true;
                }
            }
        }

        if row <= grid_size - 3 {
            let down1 = self.coords_to_id(row + 1, col);
            let down2 = self.coords_to_id(row + 2, col);
            if let (Some(s1), Some(s2)) = (state.cells.get(&down1), state.cells.get(&down2)) {
                if s1 == symbol && s2 == symbol {
                    return true;
                }
            }
        }

        false
    }

    /// Check if grid has any three-in-a-row violations
    fn has_three_in_row(&self, state: &LevelState) -> bool {
        let grid_size = self.get_grid_size();

        // Check rows
        for row in 0..grid_size {
            for col in 0..=(grid_size - 3) {
                let id1 = self.coords_to_id(row, col);
                let id2 = self.coords_to_id(row, col + 1);
                let id3 = self.coords_to_id(row, col + 2);

                if let (Some(s1), Some(s2), Some(s3)) = (
                    state.cells.get(&id1),
                    state.cells.get(&id2),
                    state.cells.get(&id3),
                ) {
                    if *s1 != Symbol::None && *s1 == *s2 && *s2 == *s3 {
                        return true;
                    }
                }
            }
        }

        // Check columns
        for col in 0..grid_size {
            for row in 0..=(grid_size - 3) {
                let id1 = self.coords_to_id(row, col);
                let id2 = self.coords_to_id(row + 1, col);
                let id3 = self.coords_to_id(row + 2, col);

                if let (Some(s1), Some(s2), Some(s3)) = (
                    state.cells.get(&id1),
                    state.cells.get(&id2),
                    state.cells.get(&id3),
                ) {
                    if *s1 != Symbol::None && *s1 == *s2 && *s2 == *s3 {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Helper functions for counting symbols
    fn count_symbol_in_row(&self, state: &LevelState, row: i32, symbol: &Symbol) -> i32 {
        let grid_size = self.get_grid_size();
        let mut count = 0;
        for col in 0..grid_size {
            let id = self.coords_to_id(row, col);
            if let Some(cell_symbol) = state.cells.get(&id) {
                if cell_symbol == symbol {
                    count += 1;
                }
            }
        }
        count
    }

    fn count_symbol_in_col(&self, state: &LevelState, col: i32, symbol: &Symbol) -> i32 {
        let grid_size = self.get_grid_size();
        let mut count = 0;

        for row in 0..grid_size {
            let id = self.coords_to_id(row, col);
            if let Some(cell_symbol) = state.cells.get(&id) {
                if cell_symbol == symbol {
                    count += 1;
                }
            }
        }

        count
    }

    /// Helper functions for coordinate conversion
    fn coords_to_id(&self, row: i32, col: i32) -> i32 {
        row * self.get_grid_size() + col
    }

    fn id_to_coords(&self, id: i32) -> (i32, i32) {
        let grid_size = self.get_grid_size();
        (id / grid_size, id % grid_size)
    }

    fn get_grid_size(&self) -> i32 {
        self.columns
    }

    /// Check if puzzle is solved
    ///
    /// Simply: Are all cells filled
    fn solved(&self, state: &LevelState) -> bool {
        state.cells.iter().all(|c| *c.1 != Symbol::None)
    }

    pub fn generate_level_from_state(&mut self, state: &LevelState) -> Gd<Level> {
        let level_scene = load::<PackedScene>(LEVEL_SCENE_PATH);
        let mut level = level_scene.instantiate_as::<Level>();
        let mut cell_props = array![];
        for id in 0..self.get_grid_size() * self.get_grid_size() {
            let symbol = state.cells.get(&id).unwrap_or(&Symbol::None).clone();
            let prop = CellProps::new(id, symbol, array![]);
            cell_props.push(&prop);
        }
        level.bind_mut().build_with_props_preview(cell_props);
        level
    }

    pub fn build_default(&self) -> Gd<Level> {
        let level_scene = load::<PackedScene>(LEVEL_SCENE_PATH);
        let mut level = level_scene.instantiate_as::<Level>();

        let mut cell_props = array![];
        for id in 0..self.columns * self.columns {
            let props = CellProps::new(id, Symbol::None, array![]);
            cell_props.push(&props);
        }
        level.bind_mut().build_with_props_preview(cell_props);
        level
    }
}
