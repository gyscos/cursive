use cursive_core::{Cursive, Printer, Vec2};
use cursive_core::direction::Direction;
use cursive_core::event::{Event, EventResult, MouseButton, MouseEvent};
use cursive_core::theme::{BaseColor, Color, ColorStyle};
use cursive_core::view::CannotFocus;
use cursive_core::views::Dialog;
use crate::board::model::{Board, CellContent, CellState, Options, RevealResult};

pub struct BoardView {
    // Actual board, unknown to the player.
    board: Board,

    focused: Option<Vec2>,
    enabled: bool,

    _missing_mines: usize,
}

impl BoardView {
    pub fn new(options: Options) -> Self {
        let board = Board::new(options);
        BoardView {
            board,
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
        self.board.toggle_flag(pos);
    }

    fn reveal(&mut self, pos: Vec2) -> EventResult {
        Self::handle_reveal_result(self.board.reveal(pos))
    }

    fn auto_reveal(&mut self, pos: Vec2) -> EventResult {
        Self::handle_reveal_result(self.board.auto_reveal(pos))
    }

    fn handle_reveal_result(reveal_result: RevealResult) -> EventResult {
        match reveal_result {
            RevealResult::Revealed => EventResult::Consumed(None),
            RevealResult::Victory => EventResult::with_cb(|s| Self::make_end_game_result(s, "Victory!")),
            RevealResult::Loss => EventResult::with_cb(|s| Self::make_end_game_result(s, "Defeted")),
        }
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
}

impl cursive::view::View for BoardView {
    fn draw(&self, printer: &Printer) {
        use CellState::*;
        use CellContent::*;

        for (i, cell) in self.board.iter().enumerate() {
            let x = (i % self.board.size.x) * 2;
            let y = i / self.board.size.x;

            let text = match (cell.state, cell.content, self.board.is_ended) {
                (Closed, _, false) => " □",
                (Marked, _, false) => " ■",
                (Opened, Free(n), false) | (_, Free(n), true) => ["  ", " 1", " 2", " 3", " 4", " 5", " 6", " 7", " 8"][n],
                (Opened, Bomb, false) | (_, Bomb, true) => "\u{01F4A3}"
            };

            let color = match (cell.state, cell.content, self.board.is_ended) {
                (Closed, _, false) => Color::RgbLowRes(3, 3, 3),
                (Marked, _, false) => Color::RgbLowRes(4, 4, 2),
                (Opened, Free(n), false) | (_, Free(n), true) => match n {
                    1 => Color::RgbLowRes(3, 5, 3),
                    2 => Color::RgbLowRes(5, 5, 3),
                    3 => Color::RgbLowRes(5, 4, 3),
                    4 => Color::RgbLowRes(5, 3, 3),
                    5 => Color::RgbLowRes(5, 2, 2),
                    6 => Color::RgbLowRes(5, 0, 1),
                    7 => Color::RgbLowRes(5, 0, 2),
                    8 => Color::RgbLowRes(5, 0, 3),
                    _ => Color::Dark(BaseColor::White),
                }
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
