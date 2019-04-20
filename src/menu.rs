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

use crate::event::Callback;
use crate::Cursive;
use crate::With;
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
            MenuItem::Delimiter => "│",
            MenuItem::Leaf(ref label, _) | MenuItem::Subtree(ref label, _) => {
                label
            }
        }
    }

    /// Returns `true` if `self` is a delimiter.
    pub fn is_delimiter(&self) -> bool {
        match *self {
            MenuItem::Delimiter => true,
            _ => false,
        }
    }

    /// Returns `true` if `self` is a leaf node.
    pub fn is_leaf(&self) -> bool {
        match *self {
            MenuItem::Leaf(_, _) => true,
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

    /// Return a mutable reference to the subtree, if applicable.
    ///
    /// Returns `None` if `self` is not a `MenuItem::Subtree`.
    pub fn as_subtree(&mut self) -> Option<&mut MenuTree> {
        match *self {
            MenuItem::Subtree(_, ref mut tree) => Some(Rc::make_mut(tree)),
            _ => None,
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

    /// Inserts an item at the given position.
    pub fn insert(&mut self, i: usize, item: MenuItem) {
        self.children.insert(i, item);
    }

    /// Inserts a delimiter at the given position.
    pub fn insert_delimiter(&mut self, i: usize) {
        self.insert(i, MenuItem::Delimiter);
    }

    /// Adds a delimiter to the end of this tree.
    pub fn add_delimiter(&mut self) {
        let i = self.children.len();
        self.insert_delimiter(i);
    }

    /// Adds a delimiter to the end of this tree - chainable variant.
    pub fn delimiter(self) -> Self {
        self.with(Self::add_delimiter)
    }

    /// Adds a actionnable leaf to the end of this tree.
    pub fn add_leaf<S, F>(&mut self, title: S, cb: F)
    where
        S: Into<String>,
        F: 'static + Fn(&mut Cursive),
    {
        let i = self.children.len();
        self.insert_leaf(i, title, cb);
    }

    /// Inserts a leaf at the given position.
    pub fn insert_leaf<S, F>(&mut self, i: usize, title: S, cb: F)
    where
        S: Into<String>,
        F: 'static + Fn(&mut Cursive),
    {
        let title = title.into();
        self.insert(i, MenuItem::Leaf(title, Callback::from_fn(cb)));
    }

    /// Adds a actionnable leaf to the end of this tree - chainable variant.
    pub fn leaf<S, F>(self, title: S, cb: F) -> Self
    where
        S: Into<String>,
        F: 'static + Fn(&mut Cursive),
    {
        self.with(|menu| menu.add_leaf(title, cb))
    }

    /// Inserts a subtree at the given position.
    pub fn insert_subtree<S>(&mut self, i: usize, title: S, tree: MenuTree)
    where
        S: Into<String>,
    {
        let title = title.into();
        let tree = MenuItem::Subtree(title, Rc::new(tree));
        self.insert(i, tree);
    }

    /// Adds a submenu to the end of this tree.
    pub fn add_subtree<S>(&mut self, title: S, tree: MenuTree)
    where
        S: Into<String>,
    {
        let i = self.children.len();
        self.insert_subtree(i, title, tree);
    }

    /// Adds a submenu to the end of this tree - chainable variant.
    pub fn subtree<S>(self, title: S, tree: MenuTree) -> Self
    where
        S: Into<String>,
    {
        self.with(|menu| menu.add_subtree(title, tree))
    }

    /// Looks for the child at the given position.
    ///
    /// Returns `None` if `i >= self.len()`.
    pub fn get_mut(&mut self, i: usize) -> Option<&mut MenuItem> {
        self.children.get_mut(i)
    }

    /// Returns the item at the given position.
    ///
    /// Returns `None` if `i > self.len()` or if the item is not a subtree.
    pub fn get_subtree(&mut self, i: usize) -> Option<&mut MenuTree> {
        self.get_mut(i).and_then(MenuItem::as_subtree)
    }

    /// Looks for a child with the given title.
    ///
    /// Returns `None` if no such label was found.
    pub fn find_item(&mut self, title: &str) -> Option<&mut MenuItem> {
        self.children
            .iter_mut()
            .find(|child| child.label() == title)
    }

    /// Looks for a subtree with the given title.
    pub fn find_subtree(&mut self, title: &str) -> Option<&mut MenuTree> {
        self.children
            .iter_mut()
            .filter(|child| child.label() == title)
            .filter_map(MenuItem::as_subtree)
            .next()
    }

    /// Returns the position of a child with the given label.
    ///
    /// Returns `None` if no such label was found.
    pub fn find_position(&mut self, title: &str) -> Option<usize> {
        self.children
            .iter()
            .position(|child| child.label() == title)
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
