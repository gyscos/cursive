use cursive::vec::Vec2;
use rand::{thread_rng, Rng};
use std::cmp::max;

#[derive(Clone, Copy)]
pub struct Options {
    pub size: Vec2,
    pub mines: usize,
}

#[derive(Clone, Copy)]
pub enum Cell {
    Bomb,
    Free(usize),
}

pub struct Board {
    pub size: Vec2,
    pub cells: Vec<Cell>,
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

        let mut board = Board {
            size: options.size,
            cells: vec![Cell::Free(0); n_cells],
        };

        for _ in 0..options.mines {
            // Find a free cell to put a bomb
            let i = loop {
                let i = thread_rng().gen_range(0, n_cells);

                if let Cell::Bomb = board.cells[i] {
                    continue;
                }

                break i;
            };

            // We know we'll go through since that's how we picked i...
            board.cells[i] = Cell::Bomb;
            // Increase count on adjacent cells

            let pos = Vec2::new(i % options.size.x, i / options.size.x);
            for p in board.neighbours(pos) {
                if let Some(&mut Cell::Free(ref mut n)) = board.get_mut(p) {
                    *n += 1;
                }
            }
        }

        board
    }

    fn get_mut(&mut self, pos: Vec2) -> Option<&mut Cell> {
        self.cell_id(pos).map(move |i| &mut self.cells[i])
    }

    pub fn cell_id(&self, pos: Vec2) -> Option<usize> {
        if pos < self.size {
            Some(pos.x + pos.y * self.size.x)
        } else {
            None
        }
    }

    pub fn neighbours(&self, pos: Vec2) -> Vec<Vec2> {
        let pos_min = pos.saturating_sub((1, 1));
        let pos_max = (pos + (2, 2)).or_min(self.size);
        (pos_min.x..pos_max.x)
            .flat_map(|x| (pos_min.y..pos_max.y).map(move |y| Vec2::new(x, y)))
            .filter(|&p| p != pos)
            .collect()
    }
}
