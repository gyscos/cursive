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
    wrap_impl!(self.view: T);

    fn wrap_find_any<'a>(&mut self, selector: &Selector,
                         mut callback: Box<FnMut(&mut Any) + 'a>) {
        match selector {
            &Selector::Id(id) if id == self.id => callback(&mut self.view),
            s => self.view.find_any(s, callback),
        }
    }
}
