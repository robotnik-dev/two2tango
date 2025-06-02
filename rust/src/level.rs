use godot::{
    classes::{Control, GridContainer, IControl},
    global::sqrt,
    prelude::*,
};

use crate::grid_cell::{CellProps, Constraint, GridCell};

pub const MIN_COLUMNS: i32 = 6;
pub const MAX_COLUMNS: i32 = 14;

#[derive(GodotClass)]
#[class(tool, init, base=Control)]
pub struct Level {
    #[export(range = (MIN_COLUMNS.into(), MAX_COLUMNS.into(), 2.))]
    #[var(get, set = set_columns)]
    #[init(val = 6)]
    pub columns: i32,

    #[export]
    grid: OnEditor<Gd<GridContainer>>,

    #[export]
    cell_scene: OnEditor<Gd<PackedScene>>,

    base: Base<Control>,
}

#[godot_api]
impl IControl for Level {
    fn ready(&mut self) {}
}

#[godot_api]
impl Level {
    #[signal]
    pub fn cell_clicked(id: i32);

    #[signal]
    pub fn level_solved();

    pub fn build(&mut self, cell_props: Vec<CellProps>) {
        self.update_cell_props(cell_props);
    }

    pub fn update_cell_props(&mut self, cell_props: Vec<CellProps>) {
        let amount = cell_props.len() as i32;
        self.set_columns(sqrt(amount as f64) as i32);
        for mut c in self.grid.get_children().iter_shared() {
            c.queue_free();
        }

        for props in cell_props {
            let mut cell = self.cell_scene.instantiate_as::<GridCell>();
            self.grid.add_child(&cell);
            cell.set_owner(&self.to_gd());
            cell.bind_mut().setup(props);
            cell.signals()
                .clicked()
                .connect_other(self, Self::on_cell_clicked);
        }
    }

    #[func]
    pub fn set_columns(&mut self, columns: i32) {
        self.columns = columns;
        self.grid.set_columns(columns);
    }

    #[func]
    /// Bubbles up the signal from each cell
    pub fn on_cell_clicked(&mut self, id: i32) {
        self.signals().cell_clicked().emit(id);
    }

    pub fn get_ids(&self) -> Array<i32> {
        let mut ids = Array::new();
        for i in 0..(self.columns * self.columns) {
            ids.push(i);
        }
        ids
    }

    #[func]
    pub fn get_cells(&self) -> Array<Gd<GridCell>> {
        let mut cells = Array::new();
        for id in self.get_ids().iter_shared() {
            if let Some(cell) = self.get_cell(id) {
                cells.push(&cell);
            }
        }
        cells
    }

    #[func]
    pub fn get_cells_in_row(&self, row: i32) -> Array<Gd<GridCell>> {
        let mut cells = Array::new();
        for id in self.get_ids().iter_shared() {
            if let Some(cell) = self.get_cell(id) {
                if let Some((r, _)) = self.get_row_col(id) {
                    if r == row {
                        cells.push(&cell);
                    }
                }
            }
        }
        cells
    }

    #[func]
    pub fn get_cells_in_col(&self, col: i32) -> Array<Gd<GridCell>> {
        let mut cells = Array::new();
        for id in self.get_ids().iter_shared() {
            if let Some(cell) = self.get_cell(id) {
                if let Some((_, c)) = self.get_row_col(id) {
                    if c == col {
                        cells.push(&cell);
                    }
                }
            }
        }
        cells
    }

    #[func]
    pub fn get_cell(&self, id: i32) -> Option<Gd<GridCell>> {
        match self.get_grid() {
            Some(grid) => grid.get_child(id).map(|c| c.cast::<GridCell>()),
            None => None,
        }
    }

    /// Get the indices of the row and the column of a cell starting with (row: 0, col: 0) in the top left
    pub fn get_row_col(&self, id: i32) -> Option<(i32, i32)> {
        self.get_cell(id).map(|_| {
            let row = id / self.columns;
            let col = id % self.columns;
            (row, col)
        })
    }

    #[func]
    pub fn get_constraint_top(&self, id: i32) -> Constraint {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_top(),
            None => Constraint::None,
        }
    }

    #[func]
    pub fn get_constraint_right(&self, id: i32) -> Constraint {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_right(),
            None => Constraint::None,
        }
    }

    #[func]
    pub fn get_constraint_bot(&self, id: i32) -> Constraint {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_bot(),
            None => Constraint::None,
        }
    }

    #[func]
    pub fn get_constraint_left(&self, id: i32) -> Constraint {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_left(),
            None => Constraint::None,
        }
    }
}
