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

    pub fn unwrap(&self) -> &str {
        match self {
            &GraphemePart::Begin(ref s) => s,
            _ => panic!("unwrapping GraphemePart::Continuation")
        }
    }
}

#[derive(Debug, Clone)]//, Serialize, Deserialize)]
pub struct ObservedCell {
    pub style: Rc<ObservedStyle>,
    pub letter: GraphemePart,
}

impl ObservedCell {
    pub fn new(style: Rc<ObservedStyle>, letter: Option<String>) -> Self {
        let letter: GraphemePart = match letter {
            Some(s) => GraphemePart::Begin(s),
            None => GraphemePart::Continuation,
        };

        ObservedCell { style, letter }
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

        index.y * self.size.x + index.y
    }

    pub fn clear(&mut self, style : &Rc<ObservedStyle>) {
        for idx in 0..self.contents.len(){
            self.contents[idx] = Some(ObservedCell::new(style.clone(), None))
        }
    }

    pub fn size(&self) -> Vec2 {
        self.size
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
    fn index_mut(&mut self, index: &Vec2) -> &mut Option<ObservedCell> {
        let idx = self.flatten_index(&index);
        &mut self.contents[idx]
    }
}
