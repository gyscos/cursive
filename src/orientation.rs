use vec::Vec2;

#[derive(Clone,Copy,PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn get(&self, v: &Vec2) -> usize {
        match *self {
            Orientation::Horizontal => v.x,
            Orientation::Vertical => v.y,
        }
    }

    pub fn swap(&self) -> Self {
        match *self {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
    }

    pub fn get_ref<'a,'b>(&'a self, v: &'b mut Vec2) -> &'b mut usize {
        match *self {
            Orientation::Horizontal => &mut v.x,
            Orientation::Vertical => &mut v.y,
        }
    }

    pub fn stack<'a,T: Iterator<Item=&'a Vec2>>(&self, iter: T) -> Vec2 {
        match *self {
            Orientation::Horizontal => iter.fold(Vec2::zero(), |a,b| a.stack_horizontal(&b)),
            Orientation::Vertical => iter.fold(Vec2::zero(), |a,b| a.stack_vertical(&b)),
        }
    }
}
