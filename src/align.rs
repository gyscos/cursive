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
