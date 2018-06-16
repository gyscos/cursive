//! Tools to control view alignment.

/// Specifies the alignment along both horizontal and vertical directions.
#[derive(Debug)]
pub struct Align {
    /// Horizontal alignment policy
    pub h: HAlign,
    /// Vertical alignment policy
    pub v: VAlign,
}

impl Align {
    /// Creates a new Align object from the given alignments.
    pub fn new(h: HAlign, v: VAlign) -> Self {
        Align { h, v }
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
#[derive(Debug)]
pub enum HAlign {
    /// Place the element to the left of available space
    Left,
    /// Place the element horizontally in the center of available space
    Center,
    /// Place the element to the right of available space
    Right,
}

/// Vertical alignment
#[derive(Debug)]
pub enum VAlign {
    /// Place the element at the top of available space
    Top,
    /// Place the element vertically in the center of available space
    Center,
    /// Place the element at the bottom of available space
    Bottom,
}

impl HAlign {
    /// Returns the offset required to position a view.
    ///
    /// When drawing a view with size `content` when the available size is
    /// `container`, printing at the resulting offset will align the view as
    /// desired.
    pub fn get_offset(&self, content: usize, container: usize) -> usize {
        if container < content {
            0
        } else {
            match *self {
                HAlign::Left => 0,
                HAlign::Center => (container - content) / 2,
                HAlign::Right => (container - content),
            }
        }
    }
}

impl VAlign {
    /// Returns the offset required to position a view.
    ///
    /// When drawing a view with size `content` when the available size is
    /// `container`, printing at the resulting offset will align the view as
    /// desired.
    pub fn get_offset(&self, content: usize, container: usize) -> usize {
        if container < content {
            0
        } else {
            match *self {
                VAlign::Top => 0,
                VAlign::Center => (container - content) / 2,
                VAlign::Bottom => (container - content),
            }
        }
    }
}
