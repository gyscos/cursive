use vec::Vec2;

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

    pub fn horizontal(&self) -> u32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> u32 {
        self.top + self.bottom
    }

    pub fn combined(&self) -> Vec2 {
        Vec2::new(self.horizontal(), self.vertical())
    }

    pub fn top_left(&self) -> Vec2 {
        Vec2::new(self.left, self.top)
    }

    pub fn bot_right(&self) -> Vec2 {
        Vec2::new(self.right, self.bottom)
    }
}
