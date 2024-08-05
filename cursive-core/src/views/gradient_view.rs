use crate::{
    style::{gradient::Interpolator, ColorStyle, Style},
    traits::With,
    Printer, Rect, Vec2, View,
};

enum CoordinateSystem {
    Local,
    Global,
}

/// Wraps a view and applies a gradient to it.
pub struct GradientView<T, I> {
    view: T,
    interpolator: I,
    coordinate: CoordinateSystem,
}

// Types of gradient:
//
// Coordinate system (flag):
// * Global
// * Local
//
// Interpolation function (trait?):
// * Linear angled + offset (angle, linear gradient)
// * Bilinear (corners, from 0-1^2)
// * Radial (center + linear gradient)
// * Composite
//
impl<T, I> GradientView<T, I> {
    /// Creates a new GradientView around the given view and interpolator.
    pub fn new(view: T, interpolator: I) -> Self {
        Self {
            view,
            interpolator,
            coordinate: CoordinateSystem::Local,
        }
    }

    /// Gives mutable access to the interpolator.
    pub fn interpolator_mut(&mut self) -> &mut I {
        &mut self.interpolator
    }

    /// Sets this gradient view to use global coordinates.
    ///
    /// The gradient will be calculated based on the absolute position in the full window.
    pub fn set_global(&mut self) {
        self.coordinate = CoordinateSystem::Global;
    }

    /// Sets this gradient view to use local coordinates (the default behaviour).
    ///
    /// The gradient will be calculated based on the relative position within the view.
    pub fn set_local(&mut self) {
        self.coordinate = CoordinateSystem::Local;
    }

    /// Sets this gradient view to use local coordinates (the default behaviour).
    ///
    /// The gradient will be calculated based on the relative position within the view.
    ///
    /// Chainable variant.
    pub fn local(self) -> Self {
        self.with(Self::set_local)
    }

    /// Sets this gradient view to use global coordinates.
    ///
    /// The gradient will be calculated based on the absolute position in the full window.
    ///
    /// Chainable variant.
    pub fn global(self) -> Self {
        self.with(Self::set_global)
    }
}

impl<T, I> GradientView<T, I>
where
    I: Interpolator,
{
    /// Gets the gradient style at the given position inside the viewport.
    ///
    /// `pos` is the relative position inside the viewport.
    /// `viewport` is the region of the terminal we are drawing into.
    /// `size` is the size of the entire terminal.
    pub fn style_at(&self, pos: Vec2, viewport: Rect, size: Vec2) -> Style {
        let (pos, size) = match self.coordinate {
            CoordinateSystem::Local => (pos, viewport.size()),
            CoordinateSystem::Global => (pos + viewport.top_left(), size),
        };

        // Multiply the X axis by 2 because most terminal fonts use cells
        // approximately twice as tall as they are wide.
        ColorStyle::back(
            self.interpolator
                .interpolate(pos * Vec2::new(1, 2), size * Vec2::new(1, 2))
                .as_color(),
        )
        .into()
    }
}

impl<T: View, I: Interpolator> crate::view::ViewWrapper for GradientView<T, I>
where
    I: Send + Sync + 'static,
{
    crate::wrap_impl!(self.view: T);

    fn wrap_draw(&self, printer: &Printer) {
        self.view.draw(printer);

        let buffer_size = printer.buffer_size();
        printer.on_window(|window| {
            let viewport = window.viewport();
            for y in 0..viewport.height() {
                for x in 0..viewport.width() {
                    let pos = Vec2::new(x, y);
                    let Some(style) = window.style_at_mut(pos) else {
                        continue;
                    };
                    let new_style = self.style_at(pos, viewport, buffer_size);
                    *style = new_style.resolve(&printer.theme.palette, *style);
                }
            }
        });
    }
}

#[crate::blueprint(GradientView::new(view, gradient))]
struct Blueprint {
    view: crate::views::BoxedView,
    gradient: crate::style::gradient::Dynterpolator,
}

crate::manual_blueprint!(with gradient, |config, context| {
    let gradient: crate::style::gradient::Dynterpolator = context.resolve(config)?;
    Ok(move |view| GradientView::new(view, gradient))
});
