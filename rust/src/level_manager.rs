use std::collections::HashMap;

use godot::{classes::SubViewport, prelude::*};

use crate::{
    grid_cell::{Constraint, ConstraintDirection, Symbol},
    level::Level,
    level_builder::RowColumn,
};

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct LevelManager {
    #[export]
    pub level_viewport: OnEditor<Gd<SubViewport>>,

    #[var]
    active_level: Option<Gd<Level>>,

    base: Base<Node>,
}

#[godot_api]
impl INode for LevelManager {}

#[godot_api]
impl LevelManager {
    #[func]
    pub fn start_level(&mut self, level: Gd<Level>) {
        let viewport = self.level_viewport.clone();
        level.clone().reparent_ex(&viewport).done();
        level
            .signals()
            .cell_clicked()
            .connect_other(self, Self::on_cell_clicked);
        level.clone().bind_mut().enable();
        self.active_level = Some(level)
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
        // FIXME: constraints do not work
        // if not -> INVALID
        // not need to visit more than the single constraint cell becaus this will run on each cell with a symbol
        for &(id, symbol) in symbols_sorted.iter() {
            for constraint_props in level.bind().get_constraint_props(*id).iter_shared() {
                // go through each cell that has a symbol and check wether it has a constraint
                match constraint_props.bind().constraint.clone() {
                    // if so, visit the cell with the constraint and check if the constraint is valid only for the row (left right)
                    Constraint::Equal => {
                        let maybe_id = match constraint_props.bind().constraint_direction {
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
                        let maybe_id = match constraint_props.bind().constraint_direction {
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
                            godot_print!(
                                "too many. id: {id} and other id: {}",
                                current_id + id_offset
                            );
                            godot_print!("symbols. id: {} symbol: {:?}", id, prev_symbol);
                            godot_print!(
                                "symbols. other id: {} symbol: {:?}",
                                current_id + id_offset,
                                symbol
                            );
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
        // FIXME: constraints do not work
        // if not -> INVALID
        // not need to visit more than the single constraint cell becaus this will run on each cell with a symbol
        for &(id, symbol) in symbols_sorted.iter() {
            for constraint_props in level.bind().get_constraint_props(*id).iter_shared() {
                // go through each cell that has a symbol and check wether it has a constraint
                match constraint_props.bind().constraint.clone() {
                    // if so, visit the cell with the constraint and check if the constraint is valid onyl for the col (top bot)
                    Constraint::Equal => {
                        let maybe_id = match constraint_props.bind().constraint_direction {
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
                        let maybe_id = match constraint_props.bind().constraint_direction {
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
