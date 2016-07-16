//! Module to build menus.

use With;
use Cursive;
use std::rc::Rc;
use event::Callback;

/// Root of a menu tree.
#[derive(Default)]
pub struct MenuTree {
    /// Menu items
    pub children: Vec<MenuItem>,
}

/// Node in the menu tree.
pub enum MenuItem {
    /// Actionnable button with a label.
    Leaf(String, Callback),
    /// Sub-menu with a label.
    Subtree(String, Rc<MenuTree>),
    /// Delimiter without a label.
    Delimiter,
}

impl MenuItem {
    /// Returns the label for this item.
    ///
    /// Returns an empty string if `self` is a delimiter.
    pub fn label(&self) -> &str {
        match *self {
            MenuItem::Delimiter => "",
            MenuItem::Leaf(ref label, _) |
            MenuItem::Subtree(ref label, _) => label,
        }
    }

    /// Returns `true` if `self` is a delimiter.
    pub fn is_delimiter(&self) -> bool {
        match *self {
            MenuItem::Delimiter => true,
            _ => false,
        }
    }

    /// Returns `true` if `self` is a subtree.
    pub fn is_subtree(&self) -> bool {
        match *self {
            MenuItem::Subtree(_, _) => true,
            _ => false,
        }
    }
}

impl MenuTree {
    /// Creates a new, empty tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of children, including delimiters.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Remove every children from this tree.
    pub fn clear(&mut self) {
        self.children.clear();
    }

    /// Returns `true` if this tree has no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Adds a delimiter to the end of this tree.
    pub fn add_delimiter(&mut self) {
        self.children.push(MenuItem::Delimiter);
    }

    /// Adds a delimiter to the end of this tree - chainable variant.
    pub fn delimiter(self) -> Self {
        self.with(|menu| menu.add_delimiter())
    }

    /// Adds a actionnable leaf to the end of this tree.
    pub fn add_leaf<F: 'static + Fn(&mut Cursive)>(&mut self, title: &str,
                                                   cb: F) {
        self.children
            .push(MenuItem::Leaf(title.to_string(), Rc::new(cb)));
    }

    /// Adds a actionnable leaf to the end of this tree - chainable variant.
    pub fn leaf<F>(self, title: &str, cb: F) -> Self
        where F: 'static + Fn(&mut Cursive)
    {
        self.with(|menu| menu.add_leaf(title, cb))
    }

    /// Adds a submenu to the end of this tree.
    pub fn add_subtree(&mut self, title: &str, tree: MenuTree) {
        self.children
            .push(MenuItem::Subtree(title.to_string(), Rc::new(tree)));
    }

    /// Adds a submenu to the end of this tree - chainable variant.
    pub fn subtree(self, title: &str, tree: MenuTree) -> Self {
        self.with(|menu| menu.add_subtree(title, tree))
    }
}
