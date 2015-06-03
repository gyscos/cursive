//! Tools to control view alignment

pub struct Align {
    pub h: HAlign,
    pub v: VAlign,
}

impl Align {
    pub fn new(h: HAlign, v: VAlign) -> Self {
        Align {
            h: h,
            v: v,
        }
    }

    pub fn top_left() -> Self {
        Align::new(HAlign::Left, VAlign::Top)
    }

    pub fn top_right() -> Self {
        Align::new(HAlign::Right, VAlign::Top)
    }

    pub fn bot_left() -> Self {
        Align::new(HAlign::Left, VAlign::Bottom)
    }

    pub fn bot_right() -> Self {
        Align::new(HAlign::Right, VAlign::Top)
    }

    pub fn center() -> Self {
        Align::new(HAlign::Center, VAlign::Center)
    }
}

pub enum HAlign {
    Left,
    Center,
    Right,
}

pub enum VAlign {
    Top,
    Center,
    Bottom,
}

impl HAlign {
    pub fn get_offset(&self, content: usize, container: usize) -> usize {
        match *self {
            HAlign::Left => 0,
            HAlign::Center => (container - content)/2,
            HAlign::Right => (container - content),
        }
    }
}

impl VAlign {
    pub fn get_offset(&self, content: usize, container: usize) -> usize {
        match *self {
            VAlign::Top => 0,
            VAlign::Center => (container - content)/2,
            VAlign::Bottom => (container - content),
        }
    }
}
