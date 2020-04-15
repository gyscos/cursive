//! View visualizing a captured PuppetBackend outputs
use crate::backends::puppet::observed::ObservedCell;
use crate::backends::puppet::observed::ObservedScreen;
use crate::theme::ColorStyle;
use crate::theme::ColorType;
use crate::view::View;
use crate::Printer;
use crate::Vec2;

/// A view that visualize observed screen
pub struct ObservedScreenView {
    screen: ObservedScreen,
}

impl ObservedScreenView {
    /// Constructor
    pub fn new(obs: ObservedScreen) -> Self {
        ObservedScreenView { screen: obs }
    }
}

impl View for ObservedScreenView {
    fn draw(&self, printer: &Printer) {
        for x in 0..self.screen.size().x {
            for y in 0..self.screen.size().y {
                let pos = Vec2::new(x, y);
                let cell_op: &Option<ObservedCell> = &self.screen[pos];
                if cell_op.is_none() {
                    continue;
                }

                let cell = cell_op.as_ref().unwrap();

                if cell.letter.is_continuation() {
                    continue;
                }

                printer.with_effects(cell.style.effects, |printer| {
                    let color_style = ColorStyle {
                        front: ColorType::Color(cell.style.colors.front),
                        back: ColorType::Color(cell.style.colors.back),
                    };

                    printer.with_color(color_style, |printer| {
                        printer.print(pos, &cell.letter.unwrap());
                    });
                });
            }
        }
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.screen.size()
    }
}
