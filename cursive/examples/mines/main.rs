mod game;

use crate::game::{AutoRevealResult, CellContent, RevealResult};
use cursive::{
    direction::Direction,
    event::{Event, EventResult, MouseButton, MouseEvent},
    theme::{BaseColor, Color, ColorStyle},
    view::CannotFocus,
    views::{Button, Dialog, LinearLayout, Panel, SelectView},
    Cursive, Printer, Vec2,
};
use std::ops::{Index, IndexMut};
use cursive_core::traits::Nameable;

fn main() {
    let mut siv = cursive::default();

    siv.add_layer(
        Dialog::new()
            .title("Minesweeper")
            .padding_lrtb(2, 2, 1, 1)
            .content(
                LinearLayout::vertical()
                    .child(Button::new_raw("   New game  ", show_options))
                    .child(Button::new_raw("   Controls  ", show_controls))
                    .child(Button::new_raw("    Scores   ", show_scores))
                    .child(Button::new_raw("     Exit    ", |s| s.quit())),
            ),
    );

    siv.run();
}

fn show_controls(s: &mut Cursive) {
    s.add_layer(Dialog::info(
        "Controls:
Reveal cell:                  left click
Mark as mine:                 right-click
Reveal nearby unmarked cells: middle-click",
    ).title("Controls"))
}

fn show_scores(s: &mut Cursive) {
    s.add_layer(Dialog::info("Not yet!").title("Scores"))
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
            .dismiss_button("Back"),
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    Unknown,
    Flag,
    Visible(usize),
    Bomb,
}

// NOTE: coordinates [y][x]
struct Overlay(Vec<Vec<Cell>>);

impl Overlay {
    pub fn new(size: Vec2) -> Self {
        Self(vec![vec![Cell::Unknown; size.x]; size.y])
    }
}

impl Index<Vec2> for Overlay {
    type Output = Cell;

    fn index(&self, pos: Vec2) -> &Self::Output {
        &self.0[pos.y][pos.x]
    }
}

impl IndexMut<Vec2> for Overlay {
    fn index_mut(&mut self, pos: Vec2) -> &mut Self::Output {
        &mut self.0[pos.y][pos.x]
    }
}

struct BoardView {
    // Actual board, unknown to the player.
    board: game::Board,

    // Visible board
    overlay: Overlay,

    focused: Option<Vec2>,
    enabled: bool,
    _missing_mines: usize,
}

impl BoardView {
    pub fn new(options: game::Options) -> Self {
        let board = game::Board::new(options);
        BoardView {
            board,
            overlay: Overlay::new(options.size),
            focused: None,
            enabled: true,
            _missing_mines: options.mines,
        }
    }

    fn get_cell(&self, mouse_pos: Vec2, offset: Vec2) -> Option<Vec2> {
        let pos = mouse_pos.checked_sub(offset)?.map_x(|x| x / 2);
        if pos.fits_in(self.board.size - (1, 1)) {
            Some(pos)
        } else {
            None
        }
    }

    fn flag(&mut self, pos: Vec2) {
        let new_cell = match self.overlay[pos] {
            Cell::Unknown => Cell::Flag,
            Cell::Flag => Cell::Unknown,
            other => other,
        };
        self.overlay[pos] = new_cell;
    }

    fn reveal(&mut self, pos: Vec2) -> EventResult {
        if self.overlay[pos] != Cell::Unknown {
            return EventResult::Consumed(None);
        }

        match self.board.reveal(pos) {
            RevealResult::Revealed(opened_cells) => {
                self.open_cells(opened_cells);
                EventResult::Consumed(None)
            }
            RevealResult::Loss => {
                self.open_all_mines();
                Self::result_loss()
            }
            RevealResult::Victory => {
                self.open_all_cells();
                Self::result_victory()
            }
        }
    }

    fn auto_reveal(&mut self, pos: Vec2) -> EventResult {
        match self.board.auto_reveal(pos) {
            AutoRevealResult::Revealed(opened_cells) => {
                self.open_cells(opened_cells);
                return EventResult::Consumed(None);
            }
            AutoRevealResult::Victory => {
                self.open_all_cells();
                Self::result_victory()
            }
        }
    }

    fn result_loss() -> EventResult {
        EventResult::with_cb(|s| Self::make_end_game_result(s, "Defeted"))
    }

    fn result_victory() -> EventResult {
        EventResult::with_cb(|s| Self::make_end_game_result(s, "Victory!"))
    }

    fn make_end_game_result(s: &mut Cursive, button_label: &'static str) {
        s.call_on_name("board", |b: &mut BoardView| b.enabled = false);
        Self::change_game_button_label(s, button_label);
    }
    fn change_game_button_label(s: &mut Cursive, label: &str) {
        s.call_on_name("game", |d: &mut Dialog| {
            d.buttons_mut().last().expect("button must exists").set_label(label);
        });
    }

    fn open_cells(&mut self, opened_cells: Vec<Vec2>) {
        for pos in opened_cells {
            let CellContent::Free(near_bombs) = self.board[pos].content else {
                panic!("must be variant CellContent::Free()")
            };

            self.overlay[pos] = Cell::Visible(near_bombs);
        }
    }

    fn open_all_cells(&mut self) {
        for pos in self.board.field.all_cell_pos_iter() {
            self.overlay[pos] = match self.board[pos].content {
                CellContent::Bomb => Cell::Bomb,
                CellContent::Free(near_bombs) => Cell::Visible(near_bombs),
            };
        }
    }

    fn open_all_mines(&mut self) {
        for pos in self.board.field.all_cell_pos_iter() {
            if let Cell::Bomb = self.overlay[pos] {
                self.overlay[pos] = Cell::Bomb;
            }
        }
    }
}

impl cursive::view::View for BoardView {
    fn draw(&self, printer: &Printer) {
        for (i, cell) in self.overlay.0.iter().flatten().enumerate() {
            let x = (i % self.board.size.x) * 2;
            let y = i / self.board.size.x;

            let text = match *cell {
                Cell::Unknown => " □",
                Cell::Flag => " ■",
                Cell::Visible(n) => ["  ", " 1", " 2", " 3", " 4", " 5", " 6", " 7", " 8"][n],
                Cell::Bomb => "\u{01F4A3} "
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
                ColorStyle::new(Color::Dark(BaseColor::Black), color),
                |printer| printer.print((x, y), text),
            );
        }
    }

    fn take_focus(&mut self, _: Direction) -> Result<EventResult, CannotFocus> {
        Ok(EventResult::Consumed(None))
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        if self.enabled {
            match event {
                Event::Mouse {
                    offset,
                    position,
                    event: MouseEvent::Press(_btn),
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
                                MouseButton::Middle => return self.auto_reveal(pos),
                                _ => (),
                            }
                        }

                        self.focused = None;
                    }
                }
                _ => (),
            }
        }

        EventResult::Ignored
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.board.size.map_x(|x| 2 * x)
    }
}

fn new_game(siv: &mut Cursive, options: game::Options) {
    let _board = game::Board::new(options);

    let dialog = Dialog::new()
        .title("Minesweeper")
        .content(LinearLayout::horizontal().child(Panel::new(BoardView::new(options).with_name("board"))))
        .button("Quit game", |s| { s.pop_layer(); })
        .with_name("game");

    siv.add_layer(dialog);
}
