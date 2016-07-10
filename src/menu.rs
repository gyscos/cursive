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

    pub fn is_subtree(&self) -> bool {
        match *self {
            MenuItem::Subtree(_, _) => true,
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

    pub fn add_delimiter(&mut self) {
        self.children.push(MenuItem::Delimiter);
    }

    pub fn delimiter(self) -> Self {
        self.with(|menu| menu.add_delimiter())
    }

    pub fn add_leaf<F: 'static + Fn(&mut Cursive)>(&mut self, title: &str,
                                                   cb: F) {
        self.children
            .push(MenuItem::Leaf(title.to_string(), Rc::new(cb)));
    }

    pub fn leaf<F: 'static + Fn(&mut Cursive)>(self, title: &str, cb: F) -> Self {
        self.with(|menu| menu.add_leaf(title, cb))
    }

    pub fn add_subtree(&mut self, title: &str, tree: MenuTree) {
        self.children
            .push(MenuItem::Subtree(title.to_string(), Rc::new(tree)));
    }

    pub fn subtree(self, title: &str, tree: MenuTree) -> Self {
        self.with(|menu| menu.add_subtree(title, tree))
    }

    pub fn with<F: FnOnce(&mut Self)>(mut self, f: F) -> Self {
        f(&mut self);
        self
    }
}
