//! Tools to control view alignment

/// Specifies the alignment along both horizontal and vertical directions.
pub struct Align {
    pub h: HAlign,
    pub v: VAlign,
}

impl Align {
    /// Creates a new Align object from the given horizontal and vertical alignments.
    pub fn new(h: HAlign, v: VAlign) -> Self {
        Align {
            h: h,
            v: v,
        }
    }

    /// Creates a top-left alignment.
    pub fn top_left() -> Self {
        Align::new(HAlign::Left, VAlign::Top)
    }

    /// Creates a top-right alignment.
    pub fn top_right() -> Self {
        Align::new(HAlign::Right, VAlign::Top)
    }

    /// Creates a bottom-left alignment.
    pub fn bot_left() -> Self {
        Align::new(HAlign::Left, VAlign::Bottom)
    }

    /// Creates a bottom-right alignment.
    pub fn bot_right() -> Self {
        Align::new(HAlign::Right, VAlign::Top)
    }

    /// Creates an alignment centered both horizontally and vertically.
    pub fn center() -> Self {
        Align::new(HAlign::Center, VAlign::Center)
    }
}

/// Horizontal alignment
pub enum HAlign {
    Left,
    Center,
    Right,
}

/// Vertical alignment
pub enum VAlign {
    Top,
    Center,
    Bottom,
}

impl HAlign {
    /// To draw a view with size `content` in a printer with size `container`, this returns the
    /// offset to start printing the view at.
    pub fn get_offset(&self, content: usize, container: usize) -> usize {
        match *self {
            HAlign::Left => 0,
            HAlign::Center => (container - content)/2,
            HAlign::Right => (container - content),
        }
    }
}

impl VAlign {
    /// To draw a view with size `content` in a printer with size `container`, this returns the
    /// offset to start printing the view at.
    pub fn get_offset(&self, content: usize, container: usize) -> usize {
        match *self {
            VAlign::Top => 0,
            VAlign::Center => (container - content)/2,
            VAlign::Bottom => (container - content),
        }
    }
}
