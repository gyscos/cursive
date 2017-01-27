//! Build menu trees.
//!
//! Menus are a way to arrange many actions in groups of more manageable size.
//!
//! A menu can be seen as a `MenuTree`. It has a list of children:
//!
//! * Leaf nodes are made of a label and a callback
//! * Sub-trees are made of a label, and another `MenuTree`.
//! * Delimiters are just there to separate groups of related children.
//!
//! The [menubar] is the main way to show menus.
//!
//! [menubar]: ../struct.Cursive.html#method.menubar

use Cursive;
use With;
use event::Callback;
use std::rc::Rc;

/// Root of a menu tree.
#[derive(Default, Clone)]
pub struct MenuTree {
    /// Menu items
    pub children: Vec<MenuItem>,
}

/// Node in the menu tree.
#[derive(Clone)]
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

    /// Remove every children from this tree.
    pub fn clear(&mut self) {
        self.children.clear();
    }

    /// Inserts a delimiter at the given position.
    pub fn insert_delimiter(&mut self, i: usize) {
        self.children.insert(i, MenuItem::Delimiter);
    }

    /// Adds a delimiter to the end of this tree.
    pub fn add_delimiter(&mut self) {
        let i = self.children.len();
        self.insert_delimiter(i);
    }

    /// Adds a delimiter to the end of this tree - chainable variant.
    pub fn delimiter(self) -> Self {
        self.with(|menu| menu.add_delimiter())
    }

    /// Adds a actionnable leaf to the end of this tree.
    pub fn add_leaf<S, F>(&mut self, title: S, cb: F)
        where S: Into<String>,
              F: 'static + Fn(&mut Cursive)
    {
        let i = self.children.len();
        self.insert_leaf(i, title, cb);
    }

    /// Inserts a leaf at the given position.
    pub fn insert_leaf<S, F>(&mut self, i: usize, title: S, cb: F)
        where S: Into<String>,
              F: 'static + Fn(&mut Cursive)
    {
        let title = title.into();
        self.children.insert(i, MenuItem::Leaf(title, Callback::from_fn(cb)));
    }


    /// Adds a actionnable leaf to the end of this tree - chainable variant.
    pub fn leaf<S, F>(self, title: S, cb: F) -> Self
        where S: Into<String>,
              F: 'static + Fn(&mut Cursive)
    {
        self.with(|menu| menu.add_leaf(title, cb))
    }

    /// Inserts a subtree at the given position.
    pub fn insert_subtree<S>(&mut self, i: usize, title: S, tree: MenuTree)
        where S: Into<String>
    {
        let title = title.into();
        let tree = MenuItem::Subtree(title, Rc::new(tree));
        self.children.insert(i, tree);
    }

    /// Adds a submenu to the end of this tree.
    pub fn add_subtree<S>(&mut self, title: S, tree: MenuTree)
        where S: Into<String>
    {
        let i = self.children.len();
        self.insert_subtree(i, title, tree);
    }

    /// Adds a submenu to the end of this tree - chainable variant.
    pub fn subtree<S>(self, title: S, tree: MenuTree) -> Self
        where S: Into<String>
    {
        self.with(|menu| menu.add_subtree(title, tree))
    }

    /// Looks for a child with the given title.
    ///
    /// Returns `None` if no such label was found.
    pub fn find_item(&mut self, title: &str) -> Option<&mut MenuItem> {
        self.children
            .iter_mut()
            .find(|child| child.label() == title)
    }

    /// Returns the position of a child with the given label.
    ///
    /// Returns `None` if no such label was found.
    pub fn find_position(&mut self, title: &str) -> Option<usize> {
        self.children
            .iter()
            .position(|child| child.label() == title)
    }

    /// Looks for a subtree child with the given label.
    ///
    /// Returns `None` if the given title was not found,
    /// or if it wasn't a subtree.
    pub fn find_subtree(&mut self, title: &str) -> Option<&mut MenuTree> {
        self.find_item(title).and_then(|item| {
            if let MenuItem::Subtree(_, ref mut tree) = *item {
                Some(Rc::make_mut(tree))
            } else {
                None
            }
        })
    }

    /// Removes the item at the given position.
    pub fn remove(&mut self, i: usize) {
        self.children.remove(i);
    }

    /// Returns the number of direct children in this node.
    ///
    /// * Includes delimiters.
    /// * Does not count nested children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Returns `true` if this tree has no children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

}
