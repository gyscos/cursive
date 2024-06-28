//! Show a status bar line at the bottom of the screen.
//!
//! This example show how to create a `StatusBarExt` trait extension.
//! A trait extension is a way to combine new functionality and an
//! existing struct, and a way to improve implementation encapsulation.
//!
//! By Joel Parker Henderson (joel@joelparkerhenderson.com)

use cursive::{
    style::Style,
    utils::span::SpannedString,
    view::{Nameable, Resizable, View},
    views::{FixedLayout, Layer, OnLayoutView, TextContent, TextContentRef, TextView},
    Cursive, Rect, Vec2,
};

pub trait StatusBarExt {
    fn status_bar(&mut self, content: impl Into<SpannedString<Style>>) -> TextContent;
    fn get_status_bar_content(&mut self) -> TextContentRef;
    fn set_status_bar_content(&mut self, content: impl Into<SpannedString<Style>>);
}

impl StatusBarExt for Cursive {
    /// Create a new status bar, set to the given content.
    fn status_bar(&mut self, content: impl Into<SpannedString<Style>>) -> TextContent {
        let text_content = TextContent::new(content);
        self.screen_mut().add_transparent_layer(
            OnLayoutView::new(
                FixedLayout::new().child(
                    Rect::from_point(Vec2::zero()),
                    Layer::new(
                        TextView::new_with_content(text_content.clone()).with_name("status"),
                    )
                    .full_width(),
                ),
                |layout, size| {
                    let rect = Rect::from_size((0, size.y - 1), (size.x, 1));
                    layout.set_child_position(0, rect);
                    layout.layout(size);
                },
            )
            .full_screen(),
        );
        text_content
    }

    fn get_status_bar_content(&mut self) -> TextContentRef {
        self.call_on_name("status", |text_view: &mut TextView| text_view.get_content())
            .expect("get_status")
    }

    fn set_status_bar_content(&mut self, content: impl Into<SpannedString<Style>>) {
        self.call_on_name("status", |text_view: &mut TextView| {
            text_view.set_content(content);
        })
        .expect("set_status")
    }
}

pub fn main() {
    let mut siv = cursive::default();
    siv.status_bar("Hello World");
    siv.run();
}
