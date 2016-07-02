use Cursive;
use std::rc::Rc;
use event::Callback;

#[derive(Default)]
pub struct MenuTree {
    pub children: Vec<MenuItem>,
}

pub enum MenuItem {
    Leaf(String, Callback),
    Subtree(String, Rc<MenuTree>),
    Delimiter,
}

impl MenuItem {
    pub fn label(&self) -> &str {
        match *self {
            MenuItem::Delimiter => "",
            MenuItem::Leaf(ref label, _) |
            MenuItem::Subtree(ref label, _) => label,
        }
    }

    pub fn is_delimiter(&self) -> bool {
        match *self {
            MenuItem::Delimiter => true,
            _ => false,
        }
    }
}

impl MenuTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn clear(&mut self) {
        self.children.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn delimiter(mut self) -> Self {
        self.children.push(MenuItem::Delimiter);
        self
    }

    pub fn leaf<F: 'static + Fn(&mut Cursive)>(mut self, title: &str, cb: F) -> Self {
        self.children
            .push(MenuItem::Leaf(title.to_string(), Rc::new(cb)));
        self
    }

    pub fn subtree(mut self, title: &str, tree: MenuTree) -> Self {
        self.children
            .push(MenuItem::Subtree(title.to_string(), Rc::new(tree)));
        self
    }
}
