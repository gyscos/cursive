use crate::view::View;
use crate::Printer;

/// Dummy view.
///
/// Doesn't print anything. Minimal size is (1,1).
#[derive(Default, Debug, Clone, Copy)]
pub struct DummyView;

impl DummyView {
    /// Create a new `DummyView`.
    pub fn new() -> Self {
        DummyView
    }
}

impl View for DummyView {
    fn draw(&self, _: &Printer) {}

    fn needs_relayout(&self) -> bool {
        false
    }
}

#[crate::recipe(DummyView::new())]
struct Recipe;

// crate::raw_recipe!(DummyView, |_config, _context| { Ok(DummyView) });
