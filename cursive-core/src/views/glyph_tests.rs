//! Standard views draw their decorations from the theme's `GlyphSet`.

use crate::{
    Printer, Vec2,
    buffer::PrintBuffer,
    direction::Orientation,
    style::GlyphSet,
    theme::Theme,
    view::{Scrollable, View},
    views::{Button, Checkbox, EditView, RadioGroup, SelectView, SliderView, TextArea, TextView},
};
use parking_lot::RwLock;
use unicode_width::UnicodeWidthStr;

fn theme(glyphs: GlyphSet) -> Theme {
    let mut theme = Theme::retro();
    theme.glyphs = glyphs;
    theme
}

/// Lays out a view in `size` and draws it, returning the text of each row.
///
/// Unprinted cells show as spaces.
fn render_in(view: &mut dyn View, size: Vec2, glyphs: GlyphSet) -> Vec<String> {
    view.layout(size);
    let mut buffer = PrintBuffer::new();
    buffer.resize(size);
    let buffer = RwLock::new(buffer);
    let theme = theme(glyphs);
    view.draw(&Printer::new(size, &theme, &buffer));
    let buffer = buffer.read();
    (0..size.y)
        .map(|y| {
            (0..size.x)
                .map(|x| match buffer.cell_text(Vec2::new(x, y)) {
                    None | Some("") => " ",
                    Some(text) => text,
                })
                .collect()
        })
        .collect()
}

/// Draws the view at its required size.
fn render(view: &mut dyn View, glyphs: GlyphSet) -> Vec<String> {
    let size = view.required_size(Vec2::new(40, 10));
    render_in(view, size, glyphs)
}

/// Check the rendered rows, and that the required size is the same in every set.
fn check(mut view: impl View, ascii: &[&str], unicode: &[&str]) {
    assert_eq!(render(&mut view, GlyphSet::Ascii), ascii, "ascii");
    assert_eq!(render(&mut view, GlyphSet::Unicode), unicode, "unicode");
    for rows in [ascii, unicode] {
        let size = view.required_size(Vec2::new(40, 10));
        assert_eq!(rows.len(), size.y);
        assert!(rows.iter().all(|r| r.width() == size.x));
    }
}

#[test]
fn checkbox() {
    check(Checkbox::new(), &["[ ]"], &["[ ]"]);
    check(Checkbox::new().checked(), &["[X]"], &["[✓]"]);
    check(Checkbox::labelled("Hi").checked(), &["[X] Hi"], &["[✓] Hi"]);
}

#[test]
fn radio_button() {
    let mut group = RadioGroup::new();
    let first = group.button(1, "A");
    let second = group.button(2, "B");
    check(first, &["(X) A"], &["(●) A"]);
    check(second, &["( ) B"], &["( ) B"]);
}

#[test]
fn button() {
    check(Button::new("Ok", |_| ()), &["<Ok>"], &["‹Ok›"]);
    check(Button::new_raw("[ Ok ]", |_| ()), &["[ Ok ]"], &["[ Ok ]"]);

    // The label is the same as before, whatever the theme.
    let mut button = Button::new("Quit", |_| ());
    assert_eq!(button.label(), "<Quit>");
    assert_eq!(button.required_size(Vec2::new(40, 10)), Vec2::new(6, 1));
    button.set_label("Escape");
    assert_eq!(button.label(), "<Escape>");
    assert_eq!(button.required_size(Vec2::new(40, 10)), Vec2::new(8, 1));
    button.set_label_raw("Raw");
    assert_eq!(button.label(), "Raw");
    assert_eq!(button.required_size(Vec2::new(40, 10)), Vec2::new(3, 1));
    button.set_label("Again");
    assert_eq!(button.label(), "<Again>");
    assert_eq!(Button::new_raw("<x>", |_| ()).label(), "<x>");
}

#[test]
fn button_centered() {
    let mut button = Button::new("Ok", |_| ());
    assert_eq!(
        render_in(&mut button, Vec2::new(8, 1), GlyphSet::Ascii),
        ["  <Ok>  "]
    );
    assert_eq!(
        render_in(&mut button, Vec2::new(8, 1), GlyphSet::Unicode),
        ["  ‹Ok›  "]
    );
}

#[test]
fn select_view_popup() {
    let make = || {
        let mut select = SelectView::new().popup();
        select.add_item("Abc", 1);
        select.add_item("De", 2);
        select
    };
    check(make(), &["<Abc>"], &[" Abc▾"]);
    // Explicit decorators win over the theme.
    check(make().decorators("(", ")"), &["(Abc)"], &["(Abc)"]);
    check(make().decorators("[[", "]]"), &["[[Abc]]"], &["[[Abc]]"]);
}

#[test]
fn edit_view() {
    let make = || EditView::new().content("ab");
    let size = Vec2::new(6, 1);
    // The cursor is on the "b" only when focused; draw the filler.
    let rows = |view: &mut EditView, glyphs| render_in(view, size, glyphs);
    assert_eq!(rows(&mut make(), GlyphSet::Ascii), ["ab____"]);
    assert_eq!(rows(&mut make(), GlyphSet::Unicode), ["ab····"]);
    // Explicit fillers win.
    assert_eq!(rows(&mut make().filler("."), GlyphSet::Unicode), ["ab...."]);
    assert_eq!(rows(&mut make().filler(" "), GlyphSet::Ascii), ["ab    "]);
}

#[test]
fn text_area_cursor() {
    // The cursor at the end of the text is drawn as the filler.
    let mut area = TextArea::new();
    assert_eq!(
        render_in(&mut area, Vec2::new(3, 1), GlyphSet::Ascii)[0],
        "_  "
    );
    assert_eq!(
        render_in(&mut area, Vec2::new(3, 1), GlyphSet::Unicode)[0],
        "·  "
    );
}

#[test]
fn slider() {
    check(
        SliderView::new(Orientation::Horizontal, 5).value(2),
        &["-- --"],
        &["── ──"],
    );
    check(
        SliderView::new(Orientation::Vertical, 3).value(0),
        &[" ", "|", "|"],
        &[" ", "│", "│"],
    );
}

#[test]
fn scrollbar() {
    let text = "a\nb\nc\nd\ne\nf";
    let mut view = TextView::new(text).scrollable();
    let size = Vec2::new(3, 3);
    let ascii = render_in(&mut view, size, GlyphSet::Ascii);
    let unicode = render_in(&mut view, size, GlyphSet::Unicode);
    // The scrollbar is in the last column.
    let column = |rows: &[String]| -> Vec<String> {
        rows.iter()
            .map(|r| r.chars().last().unwrap().to_string())
            .collect()
    };
    // The thumb is at the top, and is one cell long.
    assert_eq!(column(&ascii), ["▒", "|", "|"]);
    assert_eq!(column(&unicode), ["█", "│", "│"]);
    // The text itself is untouched.
    assert_eq!(ascii[0].chars().next(), Some('a'));
    assert_eq!(unicode[0].chars().next(), Some('a'));
}
