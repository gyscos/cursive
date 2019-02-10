#![warn(missing_docs)]

use enumset::EnumSet;
use std::ops::Index;
use std::ops::IndexMut;
use std::rc::Rc;
use theme::ColorPair;
use theme::Effect;
use theme::Style;
use Vec2;
use theme::Color;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct ObservedStyle {
    pub colors: ColorPair,
    pub effects: EnumSet<Effect>,
}

#[derive(Debug, Clone)]
pub enum GraphemePart {
    Begin(String),
    Continuation,
}

impl GraphemePart {
    pub fn is_continuation(&self) -> bool {
        match self {
            &GraphemePart::Continuation => true,
            _ => false
        }
    }

    pub fn unwrap(&self) -> String {
        match self {
            &GraphemePart::Begin(ref s) => s.clone(),
            _ => panic!("unwrapping GraphemePart::Continuation")
        }
    }
}

#[derive(Debug, Clone)]//, Serialize, Deserialize)]
pub struct ObservedCell {
    pub pos: Vec2,
    pub style: Rc<ObservedStyle>,
    pub letter: GraphemePart,
}

impl ObservedCell {
    pub fn new(pos: Vec2, style: Rc<ObservedStyle>, letter: Option<String>) -> Self {
        let letter: GraphemePart = match letter {
            Some(s) => GraphemePart::Begin(s),
            None => GraphemePart::Continuation,
        };

        ObservedCell { pos, style, letter }
    }
}

#[derive(Debug, Clone)]//, Serialize, Deserialize)]
pub struct ObservedScreen {
    size: Vec2,
    contents: Vec<Option<ObservedCell>>,
}

impl ObservedScreen {
    pub fn new(size: Vec2) -> Self {
        let contents: Vec<Option<ObservedCell>> = vec![None; size.x * size.y];

        ObservedScreen { size, contents }
    }

    fn flatten_index(&self, index: &Vec2) -> usize {
        assert!(index.x < self.size.x);
        assert!(index.y < self.size.y);

        index.x * self.size.y + index.y
    }

    fn unflatten_index(&self, index: usize) -> Vec2 {
        assert!(index < self.contents.len());

        Vec2::new(index / self.size.y, index % self.size.y)
    }

    pub fn clear(&mut self, style : &Rc<ObservedStyle>) {
        for idx in 0..self.contents.len(){
            self.contents[idx] = Some(ObservedCell::new(self.unflatten_index(idx), style.clone(), None))
        }
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }
}

pub trait ObservedPieceInterface {
    fn min(&self) -> Vec2;
    fn max(&self) -> Vec2;
    fn parent(&self) -> &ObservedScreen;
}

struct ObservedPiece<'a> {
    min : Vec2,
    max : Vec2,
    parent : &'a ObservedScreen
}

impl ObservedPieceInterface for ObservedScreen {
    fn min(&self) -> Vec2 {
        Vec2::new(0,0)
    }

    fn max(&self) -> Vec2 {
        self.size
    }

    fn parent(&self) -> &ObservedScreen {
        self
    }
}

//impl <'a, 'b> Index<&Vec2> for ObservedPieceInterface<'a> {
//    type Output = Option<ObservedCell>;
//
//    fn index(&'b self, index: &Vec2) -> &'b Self::Output {
//        assert!(self.min().x >= index.x);
//        assert!(self.max().x < index.x);
//        assert!(self.min().y >= index.y);
//        assert!(self.max().y < index.y);
//
//        &self.parent()[&(*index + self.min())]
//    }
//}

//impl IndexMut<&Vec2> for ObservedScreen {
//    fn index_mut(&mut self, index: &Vec2) -> &mut Option<ObservedCell> {
//        let idx = self.flatten_index(&index);
//        &mut self.contents[idx]
//    }
//}



//impl Index<usize> for ObservedScreen {
//    type Output = Option<[Option<ObservedCell>]>;
//
//    /// Returns line.
//    fn index(&self, index: usize) -> Self::Output {
//        if index > self.size.y {
//            None
//        } else {
//            Some(self.contents[(index*self.size.x)..((index+1) * self.size.x)])
//        }
//    }
//}

impl Index<&Vec2> for ObservedScreen {
    type Output = Option<ObservedCell>;

    fn index(&self, index: &Vec2) -> &Self::Output {
        let idx = self.flatten_index(&index);
        &self.contents[idx]
    }
}

impl IndexMut<&Vec2> for ObservedScreen {
    fn index_mut(&mut self, index: &Vec2) -> &mut Option<ObservedCell> {
        let idx = self.flatten_index(&index);
        &mut self.contents[idx]
    }
}
