//! Tools to control view alignment.

/// Specifies the alignment along both horizontal and vertical directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Align {
    /// Horizontal alignment policy
    pub h: HAlign,
    /// Vertical alignment policy
    pub v: VAlign,
}

impl Align {
    /// Creates a new Align object from the given alignments.
    pub const fn new(h: HAlign, v: VAlign) -> Self {
        Align { h, v }
    }

    /// Creates a top-left alignment.
    pub const fn top_left() -> Self {
        Align::new(HAlign::Left, VAlign::Top)
    }

    /// Creates a top-right alignment.
    pub const fn top_right() -> Self {
        Align::new(HAlign::Right, VAlign::Top)
    }

    /// Creates a top-center alignment.
    pub const fn top_center() -> Self {
        Align::new(HAlign::Center, VAlign::Top)
    }

    /// Creates a bottom-left alignment.
    pub const fn bot_left() -> Self {
        Align::new(HAlign::Left, VAlign::Bottom)
    }

    /// Creates a bottom-right alignment.
    pub const fn bot_right() -> Self {
        Align::new(HAlign::Right, VAlign::Bottom)
    }

    /// Creates a bottom-center alignment.
    pub const fn bot_center() -> Self {
        Align::new(HAlign::Center, VAlign::Bottom)
    }

    /// Creates a center-right alignment.
    pub const fn center_left() -> Self {
        Align::new(HAlign::Left, VAlign::Center)
    }

    /// Creates a center-right alignment.
    pub const fn center_right() -> Self {
        Align::new(HAlign::Right, VAlign::Center)
    }

    /// Creates an alignment centered both horizontally and vertically.
    pub const fn center() -> Self {
        Align::new(HAlign::Center, VAlign::Center)
    }
}

impl std::str::FromStr for Align {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "top_left" => Self::top_left(),
            "top_center" => Self::top_center(),
            "top_right" => Self::top_right(),
            "center_left" => Self::center_left(),
            "center" => Self::center(),
            "center_right" => Self::center_right(),
            "bot_left" | "bottom_left" => Self::bot_left(),
            "bot_center" | "bottom_center" => Self::bot_center(),
            "bot_right" | "bottom_right" => Self::bot_right(),
            _ => return Err(()),
        })
    }
}

/// Horizontal alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HAlign {
    /// Place the element to the left of available space
    Left,
    /// Place the element horizontally in the center of available space
    Center,
    /// Place the element to the right of available space
    Right,
}

impl std::str::FromStr for HAlign {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "left" | "Left" => Self::Left,
            "center" | "Center" => Self::Center,
            "right" | "Right" => Self::Right,
            _ => return Err(()),
        })
    }
}

/// Vertical alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VAlign {
    /// Place the element at the top of available space
    Top,
    /// Place the element vertically in the center of available space
    Center,
    /// Place the element at the bottom of available space
    Bottom,
}

impl std::str::FromStr for VAlign {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Top" | "top" => Self::Top,
            "Center" | "center" => Self::Center,
            "Bottom" | "bottom" | "Bot" | "bot" => Self::Bottom,
            _ => return Err(()),
        })
    }
}

impl HAlign {
    /// Returns the offset required to position a view.
    ///
    /// When drawing a view with size `content` when the available size is
    /// `container`, printing at the resulting offset will align the view as
    /// desired.
    pub const fn get_offset(&self, content: usize, container: usize) -> usize {
        if container < content {
            0
        } else {
            match *self {
                HAlign::Left => 0,
                HAlign::Center => (container - content) / 2,
                HAlign::Right => container - content,
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
    pub const fn get_offset(&self, content: usize, container: usize) -> usize {
        if container < content {
            0
        } else {
            match *self {
                VAlign::Top => 0,
                VAlign::Center => (container - content) / 2,
                VAlign::Bottom => container - content,
            }
        }
    }
}
