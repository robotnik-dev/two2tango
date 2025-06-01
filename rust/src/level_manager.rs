use std::collections::HashMap;

use godot::{
    classes::{Engine, SubViewport},
    prelude::*,
};

use crate::{
    grid_cell::Symbol,
    level::Level,
    level_generator::{self, LevelGenerator},
};

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
        if let Some(level_generator) =
            Engine::singleton().get_singleton(level_generator::SINGLETON_NAME)
        {
            let mut level = level_generator
                .cast::<LevelGenerator>()
                .bind_mut()
                .builder()
                .columns(6)
                .difficulty(level_generator::Difficulty::Normal)
                .build();
            self.level_viewport
                .add_child_ex(&level)
                .force_readable_name(true)
                .done();
            level.connect("cell_clicked", &self.base().callable("on_cell_clicked"));
            self.set_active_level(Some(level));
        }
    }
}

#[godot_api]
impl LevelManager {
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
                                c.bind_mut().set_invalid_helper(false);
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
                                        c.bind_mut().set_invalid_helper(true);
                                    },
                                );
                            }
                            RowColumn::Col(col) => {
                                level.bind().get_cells_in_col(col).iter_shared().for_each(
                                    |mut c| {
                                        c.bind_mut().set_invalid_helper(true);
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
                let symbol = cell.bind().symbol.clone();
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

        // 1. are there more than half of symbols the same
        let mut consecutive_count = 1;
        let mut current_symbol = None;
        let mut current_id = 0;
        godot_print!("Row symbols sorted: {:?}", symbols_sorted);
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
                        if consecutive_count >= columns / 2 {
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

        // get all the row contraints (Left and Right) of each cell
        // go then trough each cell that is connected to the contraint
        // mark the cell as visited to avoid cyclic checking
        // check if cell is visible
        // match the constraint and decide wheter the constraint is not correct -> INVALID
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
                let symbol = cell.bind().symbol.clone();
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

        // 1. are there more than three symbols the same
        let mut consecutive_count = 1;
        let mut current_symbol = None;
        let mut current_id = 0;
        let id_offset = level.bind().get_columns();
        godot_print!("Col symbols sorted: {:?}", symbols_sorted);
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
                        if consecutive_count >= columns / 2 {
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

        // get all the col contraints (Left and Right) of each cell
        // go then trough each cell that is connected to the contraint
        // mark the cell as visited to avoid cyclic checking
        // check if cell is visible
        // match the constraint and decide wheter the constraint is not correct -> INVALID
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
