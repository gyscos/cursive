/// Fixed margins around a rectangular view.
pub struct Margins {
    /// Left margin
    pub left: u32,
    /// Right margin
    pub right: u32,
    /// Top margin
    pub top: u32,
    /// Bottom margin
    pub bottom: u32,
}

impl Margins {
    /// Creates new margins.
    pub fn new(left: u32, right: u32, top: u32, bottom: u32) -> Self {
        Margins {
            left: left,
            right: right,
            top: top,
            bottom: bottom,
        }
    }
}
