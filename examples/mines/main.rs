extern crate cursive;
extern crate rand;

mod game;

use cursive::Cursive;
use cursive::Printer;
use cursive::direction::Direction;
use cursive::event::{Event, EventResult, MouseButton, MouseEvent};
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::vec::Vec2;
use cursive::views::{Button, Dialog, LinearLayout, Panel, SelectView};

fn main() {
    let mut siv = Cursive::new();

    siv.add_layer(
        Dialog::new()
            .title("Minesweeper")
            .padding((2, 2, 1, 1))
            .content(
                LinearLayout::vertical()
                    .child(Button::new_raw("  New game   ", show_options))
                    .child(Button::new_raw(" Best scores ", |s| {
                        s.add_layer(Dialog::info("Not yet!").title("Scores"))
                    }))
                    .child(Button::new_raw("    Exit     ", |s| s.quit())),
            ),
    );

    siv.run();
}

fn show_options(siv: &mut Cursive) {
    siv.add_layer(
        Dialog::new()
            .title("Select difficulty")
            .content(
                SelectView::new()
                    .item(
                        "Easy:      8x8,   10 mines",
                        game::Options {
                            size: Vec2::new(8, 8),
                            mines: 10,
                        },
                    )
                    .item(
                        "Medium:    16x16, 40 mines",
                        game::Options {
                            size: Vec2::new(16, 16),
                            mines: 40,
                        },
                    )
                    .item(
                        "Difficult: 24x24, 99 mines",
                        game::Options {
                            size: Vec2::new(24, 24),
                            mines: 99,
                        },
                    )
                    .on_submit(|s, option| {
                        s.pop_layer();
                        new_game(s, *option);
                    }),
            )
            .button("Back", |s| s.pop_layer()),
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Visible(usize),
    Flag,
    Unknown,
}

struct BoardView {
    // Actual board, unknown to the player.
    board: game::Board,

    // Visible board
    overlay: Vec<Cell>,

    focused: Option<Vec2>,
    missing_mines: usize,
}

impl BoardView {
    pub fn new(options: game::Options) -> Self {
        let overlay = vec![Cell::Unknown; options.size.x * options.size.y];
        let board = game::Board::new(options);
        BoardView {
            board,
            overlay,
            focused: None,
            missing_mines: options.mines,
        }
    }

    fn get_cell(&self, mouse_pos: Vec2, offset: Vec2) -> Option<Vec2> {
        mouse_pos
            .checked_sub(offset)
            .map(|pos| pos.map_x(|x| x / 2))
            .and_then(|pos| {
                if pos.fits_in(self.board.size) {
                    Some(pos)
                } else {
                    None
                }
            })
    }

    fn flag(&mut self, pos: Vec2) {
        if let Some(i) = self.board.cell_id(pos) {
            let new_cell = match self.overlay[i] {
                Cell::Unknown => Cell::Flag,
                Cell::Flag => Cell::Unknown,
                other => other,
            };
            self.overlay[i] = new_cell;
        }
    }

    fn reveal(&mut self, pos: Vec2) -> EventResult {
        if let Some(i) = self.board.cell_id(pos) {
            if self.overlay[i] != Cell::Unknown {
                return EventResult::Consumed(None);
            }

            // Action!
            match self.board.cells[i] {
                game::Cell::Bomb => {
                    return EventResult::with_cb(|s| {
                        s.add_layer(Dialog::text("BOOOM").button("Ok", |s| {
                            s.pop_layer();
                            s.pop_layer();
                        }));
                    })
                }
                game::Cell::Free(n) => {
                    self.overlay[i] = Cell::Visible(n);
                    if n == 0 {
                        // Reveal all surrounding cells
                        for p in self.board.neighbours(pos) {
                            self.reveal(p);
                        }
                    }
                }
            }
        }
        return EventResult::Consumed(None);
    }

    fn auto_reveal(&mut self, pos: Vec2) -> EventResult {
        if let Some(i) = self.board.cell_id(pos) {
            if let Cell::Visible(n) = self.overlay[i] {
                // First: is every possible cell tagged?
                let neighbours = self.board.neighbours(pos);
                let tagged = neighbours
                    .iter()
                    .filter_map(|&pos| self.board.cell_id(pos))
                    .map(|i| self.overlay[i])
                    .filter(|&cell| cell == Cell::Flag)
                    .count();
                if tagged != n {
                    return EventResult::Consumed(None);
                }

                for p in neighbours {
                    let result = self.reveal(p);
                    if result.has_callback() {
                        return result;
                    }
                }
            }
        }

        EventResult::Consumed(None)
    }
}

impl cursive::view::View for BoardView {
    fn draw(&self, printer: &Printer) {
        for (i, cell) in self.overlay.iter().enumerate() {
            let x = (i % self.board.size.x) * 2;
            let y = i / self.board.size.x;

            let text = match *cell {
                Cell::Unknown => "[]",
                Cell::Flag => "()",
                Cell::Visible(n) => {
                    ["  ", " 1", " 2", " 3", " 4", " 5", " 6", " 7", " 8"][n]
                }
            };

            let color = match *cell {
                Cell::Unknown => Color::RgbLowRes(3, 3, 3),
                Cell::Flag => Color::RgbLowRes(4, 4, 2),
                Cell::Visible(1) => Color::RgbLowRes(3, 5, 3),
                Cell::Visible(2) => Color::RgbLowRes(5, 5, 3),
                Cell::Visible(3) => Color::RgbLowRes(5, 4, 3),
                Cell::Visible(4) => Color::RgbLowRes(5, 3, 3),
                Cell::Visible(5) => Color::RgbLowRes(5, 2, 2),
                Cell::Visible(6) => Color::RgbLowRes(5, 0, 1),
                Cell::Visible(7) => Color::RgbLowRes(5, 0, 2),
                Cell::Visible(8) => Color::RgbLowRes(5, 0, 3),
                _ => Color::Dark(BaseColor::White),
            };

            printer.with_color(
                ColorStyle::new(color, Color::Dark(BaseColor::Black)),
                |printer| printer.print((x, y), text),
            );
        }
    }

    fn take_focus(&mut self, _: Direction) -> bool {
        true
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Mouse {
                offset,
                position,
                event: MouseEvent::Press(btn),
            } => {
                // Get cell for position
                if let Some(pos) = self.get_cell(position, offset) {
                    self.focused = Some(pos);
                    return EventResult::Consumed(None);
                }
            }
            Event::Mouse {
                offset,
                position,
                event: MouseEvent::Release(btn),
            } => {
                // Get cell for position
                if let Some(pos) = self.get_cell(position, offset) {
                    if self.focused == Some(pos) {
                        // We got a click here!
                        match btn {
                            MouseButton::Left => return self.reveal(pos),
                            MouseButton::Right => {
                                self.flag(pos);
                                return EventResult::Consumed(None);
                            }
                            MouseButton::Middle => {
                                return self.auto_reveal(pos);
                            }
                            _ => (),
                        }
                    }

                    self.focused = None;
                }
            }
            _ => (),
        }

        EventResult::Ignored
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.board.size.map_x(|x| 2 * x)
    }
}

fn new_game(siv: &mut Cursive, options: game::Options) {
    let board = game::Board::new(options);

    siv.add_layer(
        Dialog::new()
            .title("Minesweeper")
            .content(
                LinearLayout::horizontal()
                    .child(Panel::new(BoardView::new(options))),
            )
            .button("Quit game", |s| {
                s.pop_layer();
            }),
    );

    siv.add_layer(Dialog::info(
        "Controls:
Reveal cell:                  left click
Mark as mine:                 right-click
Reveal nearby unmarked cells: middle-click",
    ));
}
