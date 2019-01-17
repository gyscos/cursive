#![warn(missing_docs)]


use std::rc::Rc;
use theme::Style;
use Vec2;
use std::ops::IndexMut;
use std::ops::Index;


#[derive(Debug, Clone)]
pub struct ObservedCell {
    style: Rc<Style>,
    letter : String,
}

pub struct ObservedScreen {
    size : Vec2,
    contents : Vec<Option<ObservedCell>>,
}

impl ObservedScreen {

    pub fn new(size : Vec2) -> Self {
        let contents : Vec<Option<ObservedCell>> = vec![None; size.x * size.y];

        ObservedScreen {
            size,
            contents
        }
    }

    fn flatten_index(&self, index : &Vec2) -> usize {
        assert!(index.x < self.size.x);
        assert!(index.y < self.size.y);

        index.y * self.size.x + index.y
    }
}

impl Index<&Vec2> for ObservedScreen {
    type Output = Option<ObservedCell>;

    fn index(&self, index: &Vec2) -> &Self::Output {
        let idx = self.flatten_index(&index);
        &self.contents[idx]
    }
}

impl IndexMut<&Vec2> for ObservedScreen {
    fn index_mut(&mut self, index : &Vec2) -> &mut Option<ObservedCell> {
        let idx = self.flatten_index(&index);
        &mut self.contents[idx]
    }
}