use crate::view::View;
use crate::Printer;

/// Dummy view.
///
/// Doesn't print anything. Minimal size is (1,1).
pub struct DummyView;

impl View for DummyView {
    fn draw(&self, _: &Printer) {}

    fn needs_relayout(&self) -> bool {
        false
    }
}
