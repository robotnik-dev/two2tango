use std::collections::HashMap;

use godot::{
    classes::{Engine, SubViewport},
    global::randomize,
    prelude::*,
};

use crate::{
    grid_cell::{Constraint, ConstraintDirection, Symbol},
    level::Level,
    level_generator::{self, LevelDifficultySystem, LevelGenerator},
};

#[derive(Debug, Clone)]
pub struct LevelState {
    pub cells: HashMap<i32, Symbol>,
}

pub enum RowColumn {
    Row(i32),
    Col(i32),
}

#[derive(GodotClass)]
#[class(init, base=Node)]
/// Manages the creation and completion of levels.
///
/// Enforces the ruleset and provides a method to create levels in game and on the fly
pub struct LevelManager {
    #[export]
    pub level_viewport: OnEditor<Gd<SubViewport>>,

    #[export(range = (0., 10., 0.01))]
    pub difficulty: f32,

    #[var]
    active_level: Option<Gd<Level>>,

    grid_size: i32,

    base: Base<Node>,
}

#[godot_api]
impl INode for LevelManager {
    fn ready(&mut self) {
        let system = LevelDifficultySystem::new();

        // 1. get difficulty value between 0 and 10
        let difficulty = self.get_difficulty_level();

        // 2. generates a default level without constraints to solve
        match self.generate_testable_level(system.clone(), difficulty) {
            Ok(test_level) => {
                if let Some(solved_level) = self.solve_level(test_level) {
                    // 3. solved level will then be applied with X amount of visible symbols and constraints
                    match self.generate_level_and_apply_settings(solved_level, system, difficulty) {
                        Ok(actual_level) =>{
                            self.start_level(actual_level);
                        },
                        Err(err) => godot_error!("Error occurred while applying constraints and visible symbols (should not happen): {:?}", err),
                    }
                }
            }
            Err(err) => {
                godot_error!(
                    "Error occurred trying to generate a testable level (should not happen): {:?}",
                    err
                );
            }
        };
    }
}

#[godot_api]
impl LevelManager {
    /// Returns a value between 0 and 10
    pub fn get_difficulty_level(&self) -> f32 {
        // TODO: real value trhough input
        self.get_difficulty()
    }

    /// Places the level into the world to play. The last step after generating and testing the level
    pub fn start_level(&mut self, level: Gd<Level>) {
        self.level_viewport
            .add_child_ex(&level)
            .force_readable_name(true)
            .done();

        level
            .signals()
            .cell_clicked()
            .connect_other(self, Self::on_cell_clicked);

        self.set_active_level(Some(level));
    }

    /// Generates a level to test. Here we DONT put in any constraints or visible symbols!
    pub fn generate_testable_level(
        &mut self,
        system: LevelDifficultySystem,
        difficulty: f32,
    ) -> Result<Gd<Level>, String> {
        let settings = system.apply_settings(difficulty);
        self.grid_size = settings.columns;
        if let Some(level_generator) =
            Engine::singleton().get_singleton(level_generator::SINGLETON_NAME)
        {
            let mut builder = level_generator
                .cast::<LevelGenerator>()
                .bind_mut()
                .builder();
            builder.columns(settings.columns)?;

            let level = builder.build();
            Ok(level)
        } else {
            Err("Cant get LevelGenerator Singleton".into())
        }
    }

    /// Generates a level while applying settings. Returns any error while trying to generate the level.
    /// We set all the constraints and symbols in this level
    pub fn generate_level_and_apply_settings(
        &self,
        level: Gd<Level>,
        system: LevelDifficultySystem,
        difficulty: f32,
    ) -> Result<Gd<Level>, String> {
        let settings = system.apply_settings(difficulty);

        if let Some(level_generator) =
            Engine::singleton().get_singleton(level_generator::SINGLETON_NAME)
        {
            let level_with_settings = level_generator
                .cast::<LevelGenerator>()
                .bind_mut()
                .applier(level, settings)
                .apply();

            Ok(level_with_settings)
        } else {
            Err("Cant get LevelGenerator Singleton".into())
        }
    }

    /// Solve the level while setting all the symbols. Should return a different level solved level all the time
    ///
    /// THIS TAKES A WHILE TO RUN.
    fn solve_level(&self, level: Gd<Level>) -> Option<Gd<Level>> {
        randomize();
        let mut cells = HashMap::new();
        for c in level.bind().get_cells().iter_shared() {
            let id = c.bind().get_id();
            let symbol = c.bind().get_symbol();
            cells.insert(id, symbol);
        }

        let mut stack: Vec<(LevelState, i32, Vec<Symbol>)> =
            vec![(LevelState { cells }, -1, vec![])];

        while let Some((mut current_state, last_cell, mut remaining_options)) = stack.pop() {
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
                    if let Some(level_generator) =
                        Engine::singleton().get_singleton(level_generator::SINGLETON_NAME)
                    {
                        return Some(
                            level_generator
                                .cast::<LevelGenerator>()
                                .bind_mut()
                                .builder()
                                .generate_level_from_state(&current_state, self.grid_size),
                        );
                    }
                    return None; // Failed to get level generator
                } else {
                    godot_print!("ERROR: Invalid solution detected!");
                    self.print_debug_state(&current_state);
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
                if cell == -1 {
                    continue; // No valid cells left, backtrack
                }

                remaining_options = self.get_valid_symbols(&current_state, cell);
                if remaining_options.is_empty() {
                    continue; // No valid symbols for this cell, backtrack
                }

                let symbol = remaining_options.pop().unwrap();
                (cell, symbol)
            };

            // Try this symbol
            let mut new_state = current_state.clone();
            new_state.cells.insert(cell, symbol);

            // 5. SAVE BACKTRACK POINT
            if !remaining_options.is_empty() {
                stack.push((current_state, cell, remaining_options));
            }

            // Continue with this guess
            stack.push((new_state, -1, vec![]));
        }

        None // No solution found
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

    fn print_debug_state(&self, state: &LevelState) {
        let grid_size = self.get_grid_size();
        godot_print!("Grid state:");
        for row in 0..grid_size {
            for col in 0..grid_size {
                let id = self.coords_to_id(row, col);
                let symbol = state.cells.get(&id).unwrap_or(&Symbol::None);
                godot_print!("{:?} ", symbol);
            }
        }
    }

    /// Find cells that MUST be a certain symbol
    ///
    /// - Look for patterns like: X X ?  → ? must be Y
    /// - Look for patterns like: X ? X  → ? must be Y
    /// - Check if row has 3 X's already → remaining cells must be Y
    /// - Keep doing this until no more forced moves found
    fn propagate_all_constraints(&self, state: &mut LevelState) {
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
    fn choose_best_cell(&self, state: &LevelState) -> i32 {
        let grid_size = self.get_grid_size();
        let mut best_cell = -1;
        let mut min_options = usize::MAX;
        let mut max_constraint_pressure = 0;

        for id in 0..(grid_size * grid_size) {
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
                    return id;
                }

                // Priority 2 & 3: Fewest options + highest constraint pressure
                if option_count < min_options
                    || (option_count == min_options && total_pressure > max_constraint_pressure)
                {
                    best_cell = id;
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
        self.grid_size
    }

    /// Check if puzzle is solved
    ///
    /// Simply: Are all cells filled
    fn solved(&self, state: &LevelState) -> bool {
        state.cells.iter().all(|c| *c.1 != Symbol::None)
    }

    #[func]
    fn on_cell_clicked(&mut self, id: i32) {
        if let Some(level) = self.get_active_level() {
            if let Some(mut cell) = level.bind().get_cell(id) {
                match self.move_valid(level.clone(), id) {
                    Ok(_) => {
                        if self.move_solved_puzzle(level.clone()) {
                            godot_print!("Puzzle solved");
                        } else {
                            godot_print!("Valid move but not solved");
                            level.bind().get_cells().iter_shared().for_each(|mut c| {
                                c.bind_mut().set_invalid_rust(false);
                                c.bind_mut().set_disabled(false, false);
                            });
                        }
                    }
                    Err((err, row_or_col)) => {
                        godot_print!("Move Valid Err: {:?}", err);
                        match row_or_col {
                            RowColumn::Row(row) => {
                                level.bind().get_cells_in_row(row).iter_shared().for_each(
                                    |mut c| {
                                        c.bind_mut().set_invalid_rust(true);
                                    },
                                );
                            }
                            RowColumn::Col(col) => {
                                level.bind().get_cells_in_col(col).iter_shared().for_each(
                                    |mut c| {
                                        c.bind_mut().set_invalid_rust(true);
                                    },
                                );
                            }
                        }
                        // TODO: make this input blocking better
                        // disable every cell except the one who was invalid
                        level
                            .bind()
                            .get_cells()
                            .iter_shared()
                            .filter(|c| c.bind().id != cell.bind().id)
                            .for_each(|mut c| {
                                c.bind_mut().set_disabled(true, false);
                            });
                        cell.bind_mut().set_disabled(false, true);
                    }
                }
            }
        }
    }

    fn move_valid(&self, level: Gd<Level>, id: i32) -> Result<(), (String, RowColumn)> {
        if let Some((row, col)) = level.bind().get_row_col(id) {
            if let Err((err, row)) = self.row_move_valid(level.clone(), row) {
                return Err((err, RowColumn::Row(row)));
            }
            if let Err((err, col)) = self.col_move_valid(level.clone(), col) {
                return Err((err, RowColumn::Col(col)));
            }
        };

        Ok(())
    }

    fn row_move_valid(&self, level: Gd<Level>, row: i32) -> Result<(), (String, i32)> {
        // get number of visible Symbols of type "X" in row
        // get number of visible Symbols of type "Y" in row
        let columns = level.bind().get_columns();
        let mut symbols: HashMap<i32, Symbol> = HashMap::new();
        level
            .bind()
            .get_cells_in_row(row)
            .iter_shared()
            .for_each(|cell| {
                let symbol = cell.bind().get_symbol();
                symbols.insert(cell.bind().id, symbol);
            });

        // if more than half of the columns are filled with X or Y -> INVALID
        if symbols.values().filter(|&s| s == &Symbol::X).count() as i32 > columns / 2 {
            return Err((
                format!(
                    "Invalid: Too many({:?}) '{:?}' symbols in row: {:?}",
                    symbols.values().filter(|&s| s == &Symbol::X).count(),
                    Symbol::X,
                    row
                ),
                row,
            ));
        } else if symbols.values().filter(|&s| s == &Symbol::Y).count() as i32 > columns / 2 {
            return Err((
                format!(
                    "Invalid: Too many({:?}) '{:?}' symbols in row: {:?}",
                    symbols.values().filter(|&s| s == &Symbol::Y).count(),
                    Symbol::Y,
                    row
                ),
                row,
            ));
        }

        // if a symbol is more than 2 times in a row
        // Collect only the cells that have a symbol
        let mut symbols_sorted = symbols
            .iter()
            .filter(|&(_, s)| *s != Symbol::None)
            .collect::<Vec<_>>();
        symbols_sorted.sort_by_key(|&(k, _)| k);

        // 1. are there more than two symbols the same
        let mut consecutive_count = 1;
        let mut current_symbol = None;
        let mut current_id = 0;
        for &(id, symbol) in symbols_sorted.iter() {
            match current_symbol {
                None => {
                    current_symbol = Some(symbol);
                    current_id = *id;
                }
                Some(prev_symbol) => {
                    if prev_symbol == symbol {
                        // 2. are they next to each other
                        if *id == current_id + 1 {
                            consecutive_count += 1;
                        }
                        if consecutive_count > 2 {
                            return Err((
                                format!(
                                    "Invalid: more than two consecutive '{:?}' symbols in a row",
                                    symbol
                                ),
                                row,
                            ));
                        }
                    } else {
                        consecutive_count = 1;
                        current_symbol = Some(symbol);
                    }
                    current_id = *id;
                }
            }
        }
        // if not -> INVALID
        // not need to visit more than the single constraint cell becaus this will run on each cell with a symbol
        for &(id, symbol) in symbols_sorted.iter() {
            for constraint_props in level.bind().get_constraint_props(*id).iter() {
                // go through each cell that has a symbol and check wether it has a constraint
                match constraint_props.constraint.clone() {
                    // if so, visit the cell with the constraint and check if the constraint is valid only for the row (left right)
                    Constraint::Equal => {
                        let maybe_id = match constraint_props.constraint_direction {
                            ConstraintDirection::Top => None,
                            ConstraintDirection::Right => Some(id + 1),
                            ConstraintDirection::Bot => None,
                            ConstraintDirection::Left => Some(id - 1),
                            ConstraintDirection::None => None,
                        };
                        if let Some(id_to_be_visited) = maybe_id {
                            let pair_symbol = level.bind().get_symbol(id_to_be_visited).unwrap();
                            if (*symbol != Symbol::None && pair_symbol != Symbol::None)
                                && pair_symbol != *symbol
                            {
                                return Err((
                                        format!(
                                            "Invalid: Row Constraint 'Equal' is not fulfilled with cell {id} ({symbol:?}) and cell: {id_to_be_visited} ({pair_symbol:?})",
                                        ),
                                        row,
                                    ));
                            }
                        }
                    }
                    Constraint::NonEqual => {
                        let maybe_id = match constraint_props.constraint_direction {
                            ConstraintDirection::Top => None,
                            ConstraintDirection::Right => Some(id + 1),
                            ConstraintDirection::Bot => None,
                            ConstraintDirection::Left => Some(id - 1),
                            ConstraintDirection::None => None,
                        };
                        if let Some(id_to_be_visited) = maybe_id {
                            let pair_symbol = level.bind().get_symbol(id_to_be_visited).unwrap();
                            if (*symbol != Symbol::None || pair_symbol != Symbol::None)
                                && pair_symbol == *symbol
                            {
                                return Err((
                                        format!(
                                            "Invalid: Row Constraint 'Equal' is not fulfilled with cell {id} ({symbol:?}) and cell: {id_to_be_visited} ({pair_symbol:?})",
                                        ),
                                        row,
                                    ));
                            }
                        }
                    }
                    Constraint::None => {}
                }
            }
        }
        Ok(())
    }

    fn col_move_valid(&self, level: Gd<Level>, col: i32) -> Result<(), (String, i32)> {
        // get number of visible Symbols of type "X" in row
        // get number of visible Symbols of type "Y" in row
        let columns = level.bind().get_columns();
        let mut symbols: HashMap<i32, Symbol> = HashMap::new();
        level
            .bind()
            .get_cells_in_col(col)
            .iter_shared()
            .for_each(|cell| {
                let symbol = cell.bind().get_symbol();
                symbols.insert(cell.bind().id, symbol);
            });

        // if more than half of the columns are filled with X or Y -> INVALID
        if symbols.values().filter(|&s| s == &Symbol::X).count() as i32 > columns / 2 {
            return Err((
                format!(
                    "Invalid: Too many({:?}) '{:?}' symbols in col: {:?}",
                    symbols.values().filter(|&s| s == &Symbol::X).count(),
                    Symbol::X,
                    col
                ),
                col,
            ));
        } else if symbols.values().filter(|&s| s == &Symbol::Y).count() as i32 > columns / 2 {
            return Err((
                format!(
                    "Invalid: Too many({:?}) '{:?}' symbols in col: {:?}",
                    symbols.values().filter(|&s| s == &Symbol::Y).count(),
                    Symbol::Y,
                    col
                ),
                col,
            ));
        }

        // if a symbol is more than 2 times in a col
        // Collect only the cells that have a symbol
        let mut symbols_sorted = symbols
            .iter()
            .filter(|&(_, s)| *s != Symbol::None)
            .collect::<Vec<_>>();
        symbols_sorted.sort_by_key(|&(k, _)| k);

        // 1. are there more than two symbols the same
        let mut consecutive_count = 1;
        let mut current_symbol = None;
        let mut current_id = 0;
        let id_offset = level.bind().get_columns();
        for &(id, symbol) in symbols_sorted.iter() {
            match current_symbol {
                None => {
                    current_symbol = Some(symbol);
                    current_id = *id;
                }
                Some(prev_symbol) => {
                    if prev_symbol == symbol {
                        // 2. are they next to each other
                        if *id == current_id + id_offset {
                            consecutive_count += 1;
                        }
                        if consecutive_count > 2 {
                            return Err((
                                format!(
                                    "Invalid: more than two consecutive '{:?}' symbols in a col",
                                    symbol
                                ),
                                col,
                            ));
                        }
                    } else {
                        consecutive_count = 1;
                        current_symbol = Some(symbol);
                    }
                    current_id = *id;
                }
            }
        }

        // if not -> INVALID
        // not need to visit more than the single constraint cell becaus this will run on each cell with a symbol
        for &(id, symbol) in symbols_sorted.iter() {
            for constraint_props in level.bind().get_constraint_props(*id).iter() {
                // go through each cell that has a symbol and check wether it has a constraint
                match constraint_props.constraint.clone() {
                    // if so, visit the cell with the constraint and check if the constraint is valid onyl for the col (top bot)
                    Constraint::Equal => {
                        let maybe_id = match constraint_props.constraint_direction {
                            ConstraintDirection::Top => Some(id - columns),
                            ConstraintDirection::Right => None,
                            ConstraintDirection::Bot => Some(id + columns),
                            ConstraintDirection::Left => None,
                            ConstraintDirection::None => None,
                        };
                        if let Some(id_to_be_visited) = maybe_id {
                            let pair_symbol = level.bind().get_symbol(id_to_be_visited).unwrap();
                            if (*symbol != Symbol::None && pair_symbol != Symbol::None)
                                && pair_symbol != *symbol
                            {
                                return Err((
                                        format!(
                                            "Invalid: Column Constraint 'Equal' is not fulfilled with cell {id} ({symbol:?}) and cell: {id_to_be_visited} ({pair_symbol:?})",
                                        ),
                                        col,
                                    ));
                            }
                        }
                    }
                    Constraint::NonEqual => {
                        let maybe_id = match constraint_props.constraint_direction {
                            ConstraintDirection::Top => Some(id - columns),
                            ConstraintDirection::Right => None,
                            ConstraintDirection::Bot => Some(id + columns),
                            ConstraintDirection::Left => None,
                            ConstraintDirection::None => None,
                        };
                        if let Some(id_to_be_visited) = maybe_id {
                            let pair_symbol = level.bind().get_symbol(id_to_be_visited).unwrap();
                            if (*symbol != Symbol::None || pair_symbol != Symbol::None)
                                && pair_symbol == *symbol
                            {
                                return Err((
                                        format!(
                                            "Invalid: Column Constraint 'Equal' is not fulfilled with cell {id} ({symbol:?}) and cell: {id_to_be_visited} ({pair_symbol:?})",
                                        ),
                                        col,
                                    ));
                            }
                        }
                    }
                    Constraint::None => {}
                }
            }
        }
        Ok(())
    }

    /// Assuming a valid move here.
    /// Simple check if all symbols are set in the level
    fn move_solved_puzzle(&self, level: Gd<Level>) -> bool {
        level
            .bind()
            .get_cells()
            .iter_shared()
            .all(|cell| cell.bind().symbol != Symbol::None)
    }
}
