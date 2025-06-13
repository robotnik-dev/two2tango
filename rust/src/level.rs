use godot::{
    classes::{Control, GridContainer, IControl},
    global::{randi_range, sqrt},
    prelude::*,
};

use crate::grid_cell::{CellProps, ConstraintProps, GridCell, Symbol};

pub const MIN_COLUMNS: i32 = 6;
pub const MAX_COLUMNS: i32 = 10;

#[derive(GodotClass)]
#[class(tool, init, base=Control)]
pub struct Level {
    #[export(range = (MIN_COLUMNS.into(), MAX_COLUMNS.into(), 2.))]
    #[var(get, set = set_columns)]
    #[init(val = MIN_COLUMNS)]
    pub columns: i32,

    #[export]
    grid: OnEditor<Gd<GridContainer>>,

    #[export]
    cell_scene: OnEditor<Gd<PackedScene>>,

    pub cell_properties: Array<Gd<CellProps>>,

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

    pub fn build_with_props(&mut self, cell_props: Array<Gd<CellProps>>) {
        self.update_cell_props(cell_props);
    }

    pub fn enable(&mut self) {
        for mut cell in self.get_cells().iter_shared() {
            cell.bind_mut().set_disabled(false, false);
            cell.signals()
                .clicked()
                .connect_other(self, Self::on_cell_clicked);
        }
    }

    pub fn build_with_props_preview(&mut self, cell_props: Array<Gd<CellProps>>) {
        self.cell_properties = cell_props.clone();
        let amount = cell_props.len() as i32;
        self.set_columns(sqrt(amount as f64) as i32);
        for mut c in self.grid.get_children().iter_shared() {
            c.queue_free();
        }

        for props in cell_props.iter_shared() {
            let mut cell = self.cell_scene.instantiate_as::<GridCell>();
            self.grid.add_child(&cell);
            cell.bind_mut().setup(props);
            cell.bind_mut().set_disabled(true, false);
        }
    }

    pub fn update_cell_props(&mut self, cell_props: Array<Gd<CellProps>>) {
        self.cell_properties = cell_props.clone();
        let amount = cell_props.len() as i32;
        self.set_columns(sqrt(amount as f64) as i32);
        for mut c in self.grid.get_children().iter_shared() {
            c.queue_free();
        }

        for props in cell_props.iter_shared() {
            let mut cell = self.cell_scene.instantiate_as::<GridCell>();
            self.grid.add_child(&cell);
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
    /// Return the size of the level
    pub fn get_rect(&self) -> Rect2 {
        Rect2::new(self.grid.get_global_position(), self.grid.get_size())
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

    #[func]
    pub fn get_random_cell(&self) -> Option<Gd<GridCell>> {
        let random_id = randi_range(0, (self.get_ids().len() - 1) as i64);
        self.get_cell(random_id as i32)
    }

    pub fn get_symbol(&self, id: i32) -> Option<Symbol> {
        self.get_cell(id).map(|cell| cell.bind().get_symbol())
    }

    /// Get the indices of the row and the column of a cell starting with (row: 0, col: 0) in the top left
    pub fn get_row_col(&self, id: i32) -> Option<(i32, i32)> {
        self.get_cell(id).map(|_| {
            let row = id / self.columns;
            let col = id % self.columns;
            (row, col)
        })
    }

    pub fn get_constraint_props(&self, id: i32) -> Array<Gd<ConstraintProps>> {
        self.get_cell(id).unwrap().bind().get_constraint_props()
    }
}
