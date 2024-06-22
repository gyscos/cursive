mod board;

use crate::board::model::{self};
use cursive::{
    Cursive,
    Vec2, views::{Button, Dialog, LinearLayout, Panel, SelectView},
};
use board::view::BoardView;
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

fn show_options(siv: &mut Cursive) {
    siv.add_layer(
        Dialog::new()
            .title("Select difficulty")
            .content(
                SelectView::new()
                    .item(
                        "Easy:      8x8,   10 mines",
                        model::Options {
                            size: Vec2::new(8, 8),
                            mines: 10,
                        },
                    )
                    .item(
                        "Medium:    16x16, 40 mines",
                        model::Options {
                            size: Vec2::new(16, 16),
                            mines: 40,
                        },
                    )
                    .item(
                        "Difficult: 24x24, 99 mines",
                        model::Options {
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

fn new_game(siv: &mut Cursive, options: model::Options) {

    let dialog = Dialog::new()
        .title("Minesweeper")
        .content(LinearLayout::horizontal().child(Panel::new(BoardView::new(options).with_name("board"))))
        .button("Quit game", |s| { s.pop_layer(); })
        .with_name("game");

    siv.add_layer(dialog);
}
