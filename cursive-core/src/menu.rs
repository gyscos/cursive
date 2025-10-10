//! Build menu trees.
//!
//! Menus are a way to arrange many actions in groups of more manageable size.
//!
//! A menu can be seen as a [`Tree`]. It has a list of children:
//!
//! * Leaf nodes are made of a label and a callback
//! * Sub-trees are made of a label, and another `Tree`.
//! * Delimiters are just there to separate groups of related children.
//!
//! The [menubar] is the main way to show menus.
//!
//! [`Tree`]: struct.Tree.html
//! [menubar]: ../struct.Cursive.html#method.menubar

use crate::utils::markup::PlainStr;
use crate::utils::span::{SpannedStr, SpannedText as _};
use crate::{event::Callback, style::Style, utils::markup::StyledString, Cursive, With};
use std::sync::Arc;

static DELIMITER: PlainStr = PlainStr::new_with_width("│", 1);

/// Root of a menu tree.
#[derive(Default, Clone)]
pub struct Tree {
    /// Menu items
    pub children: Vec<Item>,
}

/// Node in the menu tree.
#[derive(Clone)]
pub enum Item {
    /// Actionnable button with a label.
    Leaf {
        /// Text displayed for this entry.
        label: StyledString,
        /// Callback to run when the entry is selected.
        cb: Callback,
        /// Whether this item is enabled.
        ///
        /// Disabled items cannot be selected and are displayed grayed out.
        enabled: bool,
    },

    /// Sub-menu with a label.
    Subtree {
        /// Text displayed for this entry.
        label: StyledString,
        /// Subtree under this item.
        tree: Arc<Tree>,
        /// Whether this item is enabled.
        ///
        /// Disabled items cannot be selected and are displayed grayed out.
        enabled: bool,
    },

    /// Delimiter without a label.
    Delimiter,
}

impl Item {
    /// Create a new leaf menu item.
    pub fn leaf<S, F>(label: S, cb: F) -> Self
    where
        S: Into<StyledString>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        let label = label.into();
        let cb = Callback::from_fn(cb);
        let enabled = true;
        Item::Leaf { label, cb, enabled }
    }

    /// Create a new subtree menu item.
    pub fn subtree<S>(label: S, tree: Tree) -> Self
    where
        S: Into<StyledString>,
    {
        let label = label.into();
        let tree = Arc::new(tree);
        let enabled = true;
        Item::Subtree {
            label,
            tree,
            enabled,
        }
    }

    /// Returns the label for this item.
    ///
    /// Returns a vertical bar string if `self` is a delimiter.
    pub fn label(&self) -> &str {
        match *self {
            Item::Delimiter => DELIMITER.source(),
            Item::Leaf { ref label, .. } | Item::Subtree { ref label, .. } => label.source(),
        }
    }

    /// Returns the styled lable for this item
    ///
    /// Returns a vertical bar string if `self` is a delimiter.
    pub fn styled_label(&self) -> SpannedStr<Style> {
        match *self {
            Item::Delimiter => DELIMITER.as_styled_str(),
            Item::Leaf { ref label, .. } | Item::Subtree { ref label, .. } => {
                SpannedStr::from(label)
            }
        }
    }

    /// Returns true if this item is enabled.
    ///
    /// Only labels and subtrees can be enabled. Delimiters
    pub fn is_enabled(&self) -> bool {
        match *self {
            Item::Leaf { enabled, .. } | Item::Subtree { enabled, .. } => enabled,
            Item::Delimiter => false,
        }
    }

    /// Return a disabled version of this item.
    #[must_use]
    pub fn disabled(self) -> Self {
        self.with(Self::disable)
    }

    /// Disable this item.
    ///
    /// Disabled items cannot be selected and are shown grayed out.
    ///
    /// Does not affect delimiters.
    pub fn disable(&mut self) {
        if let Item::Leaf {
            ref mut enabled, ..
        }
        | Item::Subtree {
            ref mut enabled, ..
        } = self
        {
            *enabled = false;
        }
    }

    /// Returns `true` if `self` is a delimiter.
    pub fn is_delimiter(&self) -> bool {
        matches!(*self, Item::Delimiter)
    }

    /// Returns `true` if `self` is a leaf node.
    pub fn is_leaf(&self) -> bool {
        matches!(*self, Item::Leaf { .. })
    }

    /// Returns `true` if `self` is a subtree.
    pub fn is_subtree(&self) -> bool {
        matches!(*self, Item::Subtree { .. })
    }

    /// Return a mutable reference to the subtree, if applicable.
    ///
    /// Returns `None` if `self` is not a `Item::Subtree`.
    pub fn as_subtree(&mut self) -> Option<&mut Tree> {
        match *self {
            Item::Subtree { ref mut tree, .. } => Some(Arc::make_mut(tree)),
            _ => None,
        }
    }
}

impl Tree {
    /// Creates a new, empty tree.
    pub const fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Remove every children from this tree.
    pub fn clear(&mut self) {
        self.children.clear();
    }

    /// Inserts an item at the given position.
    pub fn insert(&mut self, i: usize, item: Item) {
        self.children.insert(i, item);
    }

    /// Inserts a delimiter at the given position.
    pub fn insert_delimiter(&mut self, i: usize) {
        self.insert(i, Item::Delimiter);
    }

    /// Adds a delimiter to the end of this tree.
    pub fn add_delimiter(&mut self) {
        let i = self.children.len();
        self.insert_delimiter(i);
    }

    /// Adds a delimiter to the end of this tree - chainable variant.
    #[must_use]
    pub fn delimiter(self) -> Self {
        self.with(Self::add_delimiter)
    }

    /// Adds a actionnable leaf to the end of this tree.
    pub fn add_leaf<S, F>(&mut self, label: S, cb: F)
    where
        S: Into<StyledString>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        let i = self.children.len();
        self.insert_leaf(i, label, cb);
    }

    /// Inserts a leaf at the given position.
    pub fn insert_leaf<S, F>(&mut self, i: usize, label: S, cb: F)
    where
        S: Into<StyledString>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        let label = label.into();
        self.insert(
            i,
            Item::Leaf {
                label,
                cb: Callback::from_fn(cb),
                enabled: true,
            },
        );
    }

    /// Adds a actionnable leaf to the end of this tree - chainable variant.
    #[must_use]
    pub fn leaf<S, F>(self, label: S, cb: F) -> Self
    where
        S: Into<StyledString>,
        F: 'static + Fn(&mut Cursive) + Send + Sync,
    {
        self.with(|menu| menu.add_leaf(label, cb))
    }

    /// Inserts a subtree at the given position.
    pub fn insert_subtree<S>(&mut self, i: usize, label: S, tree: Tree)
    where
        S: Into<StyledString>,
    {
        let label = label.into();
        let tree = Item::Subtree {
            label,
            tree: Arc::new(tree),
            enabled: true,
        };
        self.insert(i, tree);
    }

    /// Adds an item to the end of this tree.
    ///
    /// Chainable variant.
    #[must_use]
    pub fn item(self, item: Item) -> Self {
        self.with(|s| s.add_item(item))
    }

    /// Adds an item to the end of this tree.
    pub fn add_item(&mut self, item: Item) {
        let i = self.children.len();
        self.insert(i, item);
    }

    /// Adds a submenu to the end of this tree.
    pub fn add_subtree<S>(&mut self, label: S, tree: Tree)
    where
        S: Into<StyledString>,
    {
        let i = self.children.len();
        self.insert_subtree(i, label, tree);
    }

    /// Adds a submenu to the end of this tree - chainable variant.
    #[must_use]
    pub fn subtree<S>(self, label: S, tree: Tree) -> Self
    where
        S: Into<StyledString>,
    {
        self.with(|menu| menu.add_subtree(label, tree))
    }

    /// Looks for the child at the given position.
    ///
    /// Returns `None` if `i >= self.len()`.
    pub fn get_mut(&mut self, i: usize) -> Option<&mut Item> {
        self.children.get_mut(i)
    }

    /// Returns the item at the given position.
    ///
    /// Returns `None` if `i > self.len()` or if the item is not a subtree.
    pub fn get_subtree(&mut self, i: usize) -> Option<&mut Tree> {
        self.get_mut(i).and_then(Item::as_subtree)
    }

    /// Looks for a child with the given label.
    ///
    /// Returns `None` if no such label was found.
    pub fn find_item(&mut self, label: &str) -> Option<&mut Item> {
        self.children
            .iter_mut()
            .find(|child| child.label() == label)
    }

    /// Looks for a subtree with the given label.
    pub fn find_subtree(&mut self, label: &str) -> Option<&mut Tree> {
        self.children
            .iter_mut()
            .filter(|child| child.label() == label)
            .find_map(Item::as_subtree)
    }

    /// Returns the position of a child with the given label.
    ///
    /// Returns `None` if no such label was found.
    pub fn find_position(&mut self, label: &str) -> Option<usize> {
        self.children
            .iter()
            .position(|child| child.label() == label)
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{ColorStyle, ColorType, Effects, Style};
    use crate::utils::span::Span;

    #[test]
    fn test_styled_label_delimiter() {
        let item = Item::Delimiter;
        let styled_label = item.styled_label();
        assert_eq!(styled_label.source(), DELIMITER.source());

        let expected_spans: Vec<Span<Style>> = vec![Span {
            content: DELIMITER.source(),
            attr: &Style {
                effects: Effects::EMPTY,
                color: ColorStyle {
                    front: ColorType::InheritParent,
                    back: ColorType::InheritParent,
                },
            },
            width: 1,
        }];

        assert_eq!(styled_label.spans().collect::<Vec<_>>(), expected_spans);
    }

    #[test]
    fn test_styled_label_leaf() {
        let label = StyledString::plain("Leaf");
        let item = Item::Leaf {
            label: label.clone(),
            enabled: true,
            cb: Callback::from_fn(|_| {}),
        };
        let styled_label = item.styled_label();
        assert_eq!(styled_label.source(), "Leaf");
    }

    #[test]
    fn test_styled_label_subtree() {
        let label = StyledString::plain("Subtree");
        let item = Item::Subtree {
            label: label.clone(),
            tree: Tree::default().into(),
            enabled: true,
        };
        let styled_label = item.styled_label();
        assert_eq!(styled_label.source(), "Subtree");
    }
}
