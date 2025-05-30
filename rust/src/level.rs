use godot::{
    classes::{Control, GridContainer, IControl},
    prelude::*,
};

use crate::grid_cell::{Constraint, GridCell};

pub const MIN_CELLS: i32 = 6;
pub const MAX_CELLS: i32 = 15;

#[derive(GodotClass)]
#[class(tool, init, base=Control)]
pub struct Level {
    #[export(range = (MIN_CELLS.into(), MAX_CELLS.into(), 1.))]
    #[var(get, set = set_columns)]
    #[init(val = 6)]
    pub columns: i32,

    #[export]
    grid: Option<Gd<GridContainer>>,

    #[export]
    cell_scene: Option<Gd<PackedScene>>,

    base: Base<Control>,
}

#[godot_api]
impl IControl for Level {
    fn ready(&mut self) {
        self.fill_grid_with_cells(self.get_columns() * self.get_columns());
    }
}

#[godot_api]
impl Level {
    #[signal]
    fn cell_clicked(id: i32);

    #[signal]
    fn level_solved();

    #[func]
    pub fn set_columns(&mut self, columns: i32) {
        self.columns = columns;
        if let Some(mut grid) = self.get_grid() {
            grid.set_columns(columns);
            self.fill_grid_with_cells(columns * columns);
        }
    }

    fn empty_grid(&mut self) {
        if let Some(grid) = self.get_grid() {
            for mut c in grid.get_children().iter_shared() {
                c.queue_free();
            }
        }
    }

    fn fill_grid_with_cells(&mut self, amount: i32) {
        self.empty_grid();
        let Some(cell_scene) = self.get_cell_scene() else {
            return;
        };
        if let Some(mut grid) = self.get_grid() {
            for id in 0..amount {
                let mut cell = cell_scene.instantiate_as::<GridCell>();
                grid.add_child(&cell);
                cell.set_owner(&self.to_gd());
                cell.connect("clicked", &self.to_gd().callable("on_cell_clicked"));
                cell.bind_mut().id = id;
            }
        }
    }

    #[func]
    pub fn on_cell_clicked(&mut self, id: i32) {
        if let Some(cell) = self.get_cell(id) {
            if self.solved() {
                self.base_mut().emit_signal("level_solved", &[]);
            } else if self.forbidden() {
                godot_print!("forbidden move with this cell: {:?}", cell);
            }
        }
    }

    #[func]
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
    pub fn get_cell(&self, id: i32) -> Option<Gd<GridCell>> {
        match self.get_grid() {
            Some(grid) => grid.get_child(id).map(|c| c.cast::<GridCell>()),
            None => None,
        }
    }

    #[func]
    pub fn get_constraint_top(&self, id: i32) -> GString {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_top(),
            None => Constraint::None.to_godot(),
        }
    }

    #[func]
    pub fn get_constraint_right(&self, id: i32) -> GString {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_right(),
            None => Constraint::None.to_godot(),
        }
    }

    #[func]
    pub fn get_constraint_bot(&self, id: i32) -> GString {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_bot(),
            None => Constraint::None.to_godot(),
        }
    }

    #[func]
    pub fn get_constraint_left(&self, id: i32) -> GString {
        match self.get_cell(id) {
            Some(cell) => cell.bind().get_constraint_left(),
            None => Constraint::None.to_godot(),
        }
    }

    fn solved(&self) -> bool {
        //TODO
        false
    }

    fn forbidden(&self) -> bool {
        //TODO
        false
    }
}
