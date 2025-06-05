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

#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    id: i32,
    symbol: Symbol,
}

impl Move {
    pub fn new(id: i32, symbol: Symbol) -> Move {
        Self { id, symbol }
    }
}

enum RowColumn {
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

    #[var]
    active_level: Option<Gd<Level>>,

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
                let solved_level = self.solve_level(test_level);

                // 3. solved level will then be applied with X amount of visible symbols and constraints
                match self.generate_level_and_apply_settings(solved_level, system, difficulty) {
                    Ok(actual_level) =>{
                        self.start_level(actual_level);
                    },
                    //TODO: remove result
                    Err(err) => godot_error!("Error occurred while applying constraints and visible symbols (should not happen): {:?}", err),
                }
            }
            Err(err) => {
                // TODO: remove result
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
        0.0
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
        &self,
        system: LevelDifficultySystem,
        difficulty: f32,
    ) -> Result<Gd<Level>, String> {
        let settings = system.apply_settings(difficulty);

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
    fn solve_level(&self, level: Gd<Level>) -> Gd<Level> {
        //TODO: make a timeout and decide what to do after the timeout
        randomize();

        let mut level_solved = false;
        let mut moves = vec![];
        let mut last_made_move = None;
        while !level_solved {
            let mut maybe_valid_move = None;
            'outer: while maybe_valid_move.is_none() {
                maybe_valid_move = self.find_next_valid_move(level.clone());
                if maybe_valid_move.is_some() {
                    // if the new move would be the same as the last saved moves that means it tries to reapply a failing strategy so we start anew
                    if let Some(last_move) = last_made_move.clone() {
                        if maybe_valid_move.clone().unwrap() == last_move {
                            moves.clear();
                            self.apply_moves_to_level(level.clone(), moves.clone());
                        }
                    }
                    break 'outer;
                }
                // go one move back
                moves.pop();
                self.apply_moves_to_level(level.clone(), moves.clone());
            }
            let valid_move = maybe_valid_move.unwrap();
            self.make_move(valid_move.clone(), level.clone());
            moves.push(valid_move.clone());
            last_made_move = Some(valid_move);
            if self.level_solved(level.clone()) {
                level_solved = true;
            }
        }
        level
    }

    /// Apply all the moves in the list to the level and set every other cell to Symbol::None
    fn apply_moves_to_level(&self, level: Gd<Level>, moves: Vec<Move>) {
        for mut cell in level.bind().get_cells().iter_shared() {
            cell.bind_mut().switch_to(Symbol::None);
        }
        for m in moves {
            level
                .bind()
                .get_cell(m.id)
                .unwrap()
                .bind_mut()
                .switch_to(m.symbol);
        }
    }

    fn level_solved(&self, level: Gd<Level>) -> bool {
        level.bind().solved()
    }

    /// Return the first found valid move or None when level is solved
    fn find_next_valid_move(&self, level: Gd<Level>) -> Option<Move> {
        let mut cells = level.bind().get_cells();
        cells.shuffle();
        for mut cell in cells
            .iter_shared()
            .filter(|c| c.bind().get_symbol() == Symbol::None)
        {
            let id = cell.bind().id;
            let m = Move::new(id, Symbol::X);
            cell.bind_mut().switch_to(m.symbol.clone());
            let maybe_move = match self.move_valid(level.clone(), id) {
                Ok(_) => {
                    cell.bind_mut().switch_to(Symbol::None);
                    Some(m)
                }
                Err(_) => {
                    cell.bind_mut().switch_to(Symbol::None);
                    None
                }
            };

            if maybe_move.is_some() {
                return maybe_move;
            };

            let m = Move::new(id, Symbol::Y);
            cell.bind_mut().switch_to(m.symbol.clone());
            let maybe_move = match self.move_valid(level.clone(), id) {
                Ok(_) => {
                    cell.bind_mut().switch_to(Symbol::None);
                    Some(m)
                }
                Err(_) => {
                    cell.bind_mut().switch_to(Symbol::None);
                    None
                }
            };

            if maybe_move.is_some() {
                return maybe_move;
            }
        }
        None
    }

    /// Makes a move on the level (switching symbols of single cells)
    fn make_move(&self, m: Move, level: Gd<Level>) {
        if let Some(mut cell) = level.bind().get_cell(m.id) {
            cell.bind_mut().switch_to(m.symbol);
        }
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
                        godot_print!("{:?}", err);
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
