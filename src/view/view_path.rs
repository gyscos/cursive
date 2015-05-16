/// Represents a path to a single view in the layout.
pub struct ViewPath {
    /// List of turns to make on decision nodes when descending the view tree.
    /// Simple nodes (with one fixed child) are skipped.
    pub path: Vec<usize>,
}

impl ViewPath {
    /// Creates a new empty path.
    pub fn new() -> Self {
        ViewPath {
            path: Vec::new(),
        }
    }

    pub fn from<T: ToPath>(path: T) -> Self {
        path.to_path()
    }
}

pub trait ToPath {
    fn to_path(self) -> ViewPath;
}

impl <'a> ToPath for &'a [usize] {
    fn to_path(self) -> ViewPath {
        ViewPath {
            path: self.to_owned(),
        }
    }
}
