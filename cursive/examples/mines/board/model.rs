use rand::seq::SliceRandom;
use std::ops::{Index, IndexMut};
use std::slice::Iter;
use ahash::AHashSet;
use cursive::{Rect, Vec2};
use crate::board::model::CellState::*;

#[derive(Clone, Copy)]
pub struct Options {
    pub size: Vec2,
    pub mines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellContent {
    Bomb,
    // numer of near bombs
    Free(usize),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CellState {
    Closed,
    // Marked by user, that this cell contains bomb
    Marked,
    Opened,
}

#[derive(Copy, Clone)]
pub struct Cell {
    pub state: CellState,

    pub content: CellContent,
}

impl Cell {
    pub fn new(content: CellContent) -> Self {
        Self {
            state: Closed,
            content,
        }
    }
}

pub struct Field {
    size: Vec2,
    cells: Vec<Cell>,
}

impl Field {
    fn new(size: Vec2) -> Self {
        Self {
            size,
            // init stub for cells, see method `Field::place_bombs()` details
            cells: vec![Cell::new(CellContent::Free(0)); size.x * size.y],
        }
    }

    fn pos_to_cell_idx(&self, pos: Vec2) -> usize {
        pos.x + pos.y * self.size.x
    }

    fn place_bombs(&mut self, click_pos: Vec2, bombs_count: usize) {
        // For avoiding losing on first player's move we should place bombs excluding
        // position where player clicked and it's neighbours

        // calculation cells from starting rect
        let rect = self.neighbours_rect(click_pos);
        let exclusion_cells: Vec<_> = (rect.top()..rect.bottom())
            .flat_map(|y| (rect.left()..rect.right()).map(move |x| Vec2::new(x, y)))
            .collect();

        // init bombs on board
        let n_cells = self.cells.len();
        self.cells.clear();
        self.cells.resize(bombs_count, Cell:: new(CellContent::Bomb));
        self.cells.resize(n_cells - exclusion_cells.len(), Cell:: new(CellContent::Free(0)));
        let mut rng = rand::thread_rng();
        self.cells.shuffle(&mut rng);

        // push empty cells near of cursor to avoid bombs in this positions
        for pos in exclusion_cells {
            self.cells.insert(self.pos_to_cell_idx(pos), Cell::new(CellContent::Free(0)));
        }

        // recalculate near bombs
        for pos in self.all_cell_pos_iter() {
            if let CellContent::Free(_) = self[pos].content {
                self[pos].content = CellContent::Free(self.calc_neighbors_bomb_count(pos));
            }
        }
    }

    pub fn all_cell_pos_iter(&self) -> impl Iterator<Item=Vec2> {
        let size = self.size;
        (0..size.y).flat_map(move |x| (0..size.x).map(move |y| Vec2::new(y, x)))
    }

    fn neighbours_rect(&self, pos: Vec2) -> Rect {
        let pos_min = pos.saturating_sub((1, 1));
        let pos_max = (pos + (2, 2)).or_min(self.size);

        Rect::from_corners(pos_min, pos_max)
    }

    fn neighbours(&self, pos: Vec2) -> impl Iterator<Item=Vec2> {
        let pos_min = pos.saturating_sub((1, 1));
        let pos_max = (pos + (2, 2)).or_min(self.size);

        (pos_min.y..pos_max.y)
            .flat_map(move |x| (pos_min.x..pos_max.x).map(move |y| Vec2::new(y, x)))
            .filter(move |&p| p != pos)
    }

    fn calc_neighbors_bomb_count(&self, cell_pos: Vec2) -> usize {
        let mut bombs_count = 0;
        for near_pos in self.neighbours(cell_pos) {
            if self[near_pos].content == CellContent::Bomb {
                bombs_count += 1;
            }
        }

        bombs_count
    }
}

impl Index<Vec2> for Field {
    type Output = Cell;

    fn index(&self, pos: Vec2) -> &Self::Output {
        &self.cells[self.pos_to_cell_idx(pos)]
    }
}

impl IndexMut<Vec2> for Field {
    fn index_mut(&mut self, pos: Vec2) -> &mut Self::Output {
        let idx = self.pos_to_cell_idx(pos);
        &mut self.cells[idx]
    }
}


pub struct Board {
    pub size: Vec2,
    pub bombs_count: usize,
    pub field: Field,
    is_bomb_placed: bool,
    pub is_ended: bool,
}

impl Board {
    pub fn new(options: Options) -> Self {
        let n_cells = options.size.x * options.size.y;
        if options.mines > n_cells {
            // Something is wrong here...
            // Use different options instead.
            return Board::new(Options {
                size: options.size,
                mines: n_cells,
            });
        }

        Board {
            size: options.size,
            bombs_count: options.mines,
            is_bomb_placed: false,
            field: Field::new(options.size),
            is_ended: false,
        }
    }

    pub fn iter(&self) -> Iter<'_, Cell>
    {
        self.field.cells.iter()
    }

    fn place_bombs_if_needed(&mut self, pos: Vec2) {
        if !self.is_bomb_placed {
            self.field.place_bombs(pos, self.bombs_count);

            self.is_bomb_placed = true;
        }
    }

    pub fn reveal(&mut self, pos: Vec2) -> RevealResult {
        self.place_bombs_if_needed(pos);

        let cell = &mut self.field[pos];
        match cell.content {
            CellContent::Bomb => {
                self.is_ended = true;

                RevealResult::Loss
            }
            CellContent::Free(_) => {
                cell.state = Opened;

                self.auto_reveal(pos)
            }
        }
    }

    pub(crate) fn auto_reveal(&mut self, pos: Vec2) -> RevealResult {
        self.place_bombs_if_needed(pos);


        let mut opened = AHashSet::new();
        if let CellContent::Free(n) = self.field[pos].content {
            let market_cells = self.field.neighbours(pos).filter(|pos| self.field[*pos].state == Marked).count();
            if market_cells == n {
                for near_pos in self.field.neighbours(pos) {
                    if !self.check_neighbours_for_auto_reveal(near_pos, &mut opened) {
                        return RevealResult::Loss;
                    }
                }
            }
        }

        match self.check_victory() {
            true => {
                self.is_ended = true;
                RevealResult::Victory
            }
            false => RevealResult::Revealed
        }
    }

    pub(crate) fn toggle_flag(&mut self, pos: Vec2) {
        self.place_bombs_if_needed(pos);

        let cell = &mut self.field[pos];
        cell.state = match cell.state {
            Closed => Marked,
            Marked => Closed,
            Opened => Opened,
        }
    }

    // NOTE: Returned when was bomb
    fn check_neighbours_for_auto_reveal(&mut self, pos: Vec2, opened: &mut AHashSet<Vec2>) -> bool {
        if self.field[pos].state == Opened || opened.contains(&pos) {
            return true;
        }

        self.field[pos].state = Opened;
        opened.insert(pos);

        match self.field[pos].content {
            CellContent::Bomb => false,
            CellContent::Free(0) => {
                for pos in self.field.neighbours(pos) {
                    if !self.check_neighbours_for_auto_reveal(pos, opened) {
                        return false;
                    }
                }
                true
            }
            _ => true,
        }
    }

    fn check_victory(&self) -> bool {
        self.field.cells.iter().filter(|x| matches!(x.content, CellContent::Free(_))).all(|x| x.state == Opened)
    }
}

impl Index<Vec2> for Board {
    type Output = Cell;

    fn index(&self, pos: Vec2) -> &Self::Output {
        &self.field[pos]
    }
}

pub enum RevealResult {
    Revealed,
    Loss,
    Victory,
}