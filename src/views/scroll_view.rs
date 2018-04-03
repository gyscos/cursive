use view::View;
use Printer;
use vec::Vec2;

/// Wraps a view in a scrollable area.
pub struct ScrollView<V> {
    inner: V,
    offset: Vec2,

    // Togglable horizontal/vertical scrolling?
}

impl <V> View for ScrollView<V> where V: View {

    fn draw(&self, printer: &Printer) {
        // Draw content
        let printer = printer.content_offset(self.offset);
        self.inner.draw(&printer);

        // Draw scrollbar?
    }
}
