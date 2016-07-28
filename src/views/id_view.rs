use std::any::Any;

use view::{Selector, View, ViewWrapper};

/// Wrapper view that allows to select its content with a fixed string id.
pub struct IdView<T: View> {
    view: T,
    id: String,
}

impl<T: View> IdView<T> {
    /// Wraps the given view. It will be selectable with the given id.
    pub fn new(id: &str, view: T) -> Self {
        IdView {
            view: view,
            id: id.to_string(),
        }
    }
}

impl<T: View + Any> ViewWrapper for IdView<T> {
    wrap_impl!(&self.view);

    fn wrap_find(&mut self, selector: &Selector) -> Option<&mut Any> {
        match selector {
            &Selector::Id(id) if id == self.id => Some(&mut self.view),
            s => self.view.find(s),
        }
    }
}
