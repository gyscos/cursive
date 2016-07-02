//! Define an Orientation and associated methods.
use vec::Vec2;

/// Describes a vertical or horizontal orientation for a view.
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum Orientation {
    /// Horizontal orientation
    Horizontal,
    /// Vertical orientation
    Vertical,
}

impl Orientation {
    /// Returns the component of the given vector corresponding to this orientation.
    /// (Horizontal will return the x value, and Vertical will return the y value.)
    pub fn get(&self, v: &Vec2) -> usize {
        match *self {
            Orientation::Horizontal => v.x,
            Orientation::Vertical => v.y,
        }
    }

    /// Returns the other orientation.
    pub fn swap(&self) -> Self {
        match *self {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
    }

    /// Returns a mutable reference to the component of the given vector
    /// corresponding to this orientation.
    pub fn get_ref<'a, 'b>(&'a self, v: &'b mut Vec2) -> &'b mut usize {
        match *self {
            Orientation::Horizontal => &mut v.x,
            Orientation::Vertical => &mut v.y,
        }
    }

    /// Takes an iterator on sizes, and stack them in the current orientation,
    /// returning the size of the required bounding box.
    ///
    /// For an horizontal view, returns (Sum(x), Max(y)).
    /// For a vertical view, returns (Max(x),Sum(y)).
    pub fn stack<'a, T: Iterator<Item = &'a Vec2>>(&self, iter: T) -> Vec2 {
        match *self {
            Orientation::Horizontal => iter.fold(Vec2::zero(), |a, b| a.stack_horizontal(b)),
            Orientation::Vertical => iter.fold(Vec2::zero(), |a, b| a.stack_vertical(b)),
        }
    }
}
