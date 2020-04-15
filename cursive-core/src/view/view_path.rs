/// Represents a path to a single view in the layout.
pub struct ViewPath {
    /// List of turns to make on decision nodes when descending the view tree.
    /// Simple nodes (with one fixed child) are skipped.
    pub path: Vec<usize>,
}

new_default!(ViewPath);

impl ViewPath {
    /// Creates a new empty path.
    pub fn new() -> Self {
        ViewPath { path: Vec::new() }
    }

    /// Creates a path from the given item.
    pub fn from<T: ToPath>(path: T) -> Self {
        path.to_path()
    }
}

/// Generic trait for elements that can be converted into a `ViewPath`.
pub trait ToPath {
    /// Creates a path from the element.
    fn to_path(self) -> ViewPath;
}

impl<'a> ToPath for &'a [usize] {
    fn to_path(self) -> ViewPath {
        ViewPath {
            path: self.to_owned(),
        }
    }
}
