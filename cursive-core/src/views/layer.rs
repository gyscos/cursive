use crate::style::ColorStyle;
use crate::view::{View, ViewWrapper};
use crate::Printer;

/// Wrapper view that fills the background.
///
/// This is mostly used as layer in the [`StackView`].
///
/// [`StackView`]: crate::views::StackView
#[derive(Debug)]
pub struct Layer<T> {
    view: T,
    color: ColorStyle,
}

new_default!(Layer<T: Default>);

impl<T> Layer<T> {
    /// Wraps the given view.
    pub fn new(view: T) -> Self {
        Self::with_color(view, ColorStyle::view())
    }

    /// Wraps the given view with a custom background color.
    pub fn with_color(view: T, color: ColorStyle) -> Self {
        Layer { view, color }
    }

    /// Gets the current color.
    pub fn color(&self) -> ColorStyle {
        self.color
    }

    /// Sets the background color.
    pub fn set_color(&mut self, color: ColorStyle) {
        self.color = color;
    }

    inner_getters!(self.view: T);
}

impl<T: View> ViewWrapper for Layer<T> {
    wrap_impl!(self.view: T);

    fn wrap_draw(&self, printer: &Printer) {
        printer.with_color(self.color, |printer| {
            for y in 0..printer.size.y {
                printer.print_hline((0, y), printer.size.x, " ");
            }
            self.view.draw(printer);
        });
    }
}

#[crate::blueprint(Layer::new(view))]
struct Blueprint {
    view: crate::views::BoxedView,
    color: Option<ColorStyle>,
}

crate::manual_blueprint!(with layer, |config, context| {
    let color = match config {
        crate::builder::Config::Null => None,
        config => Some(context.resolve(config)?),
    };
    Ok(move |view| {
        let mut layer = Layer::new(view);

        if let Some(color) = color {
            layer.set_color(color);
        }

        layer
    })
});
