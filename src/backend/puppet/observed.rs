#![warn(missing_docs)]

use enumset::EnumSet;
use std::ops::Index;
use std::ops::IndexMut;
use std::rc::Rc;
use std::string::ToString;
use theme::ColorPair;
use theme::Effect;
use theme::Style;
use Vec2;
use theme::Color;
use serde::{Serialize, Deserialize};
use unicode_segmentation::UnicodeSegmentation;
use core::borrow::Borrow;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObservedStyle {
    pub colors: ColorPair,
    pub effects: EnumSet<Effect>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

    pub fn as_option(&self) -> Option<&String> {
        match self {
            &GraphemePart::Begin(ref String) => Some(String),
            &GraphemePart::Continuation => None
        }
    }

    pub fn unwrap(&self) -> String {
        match self {
            &GraphemePart::Begin(ref s) => s.clone(),
            _ => panic!("unwrapping GraphemePart::Continuation")
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]//, Serialize, Deserialize)]
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

        index.y * self.size.x + index.x
    }

    fn unflatten_index(&self, index: usize) -> Vec2 {
        assert!(index < self.contents.len());

        Vec2::new(index / self.size.x, index % self.size.x)
    }

    pub fn clear(&mut self, style : &Rc<ObservedStyle>) {
        for idx in 0..self.contents.len(){
            self.contents[idx] = Some(ObservedCell::new(self.unflatten_index(idx), style.clone(), None))
        }
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn piece(&self, min : Vec2, max : Vec2) -> ObservedPiece {
        ObservedPiece::new(self, min, max)
    }

    pub fn find_occurences<'a>(&'a self, pattern : &str) -> Vec<ObservedLine<'a>> {
        // TODO(njskalski): make this implementation less naive?
        // TODO(njskalski): test for two-cell letters.

        let mut hits : Vec<ObservedLine> = vec![];
        for y in self.min().y..self.max().y {
            'x: for x in self.min().x..self.max().x {
                // check candidate.

                if pattern.len() > self.size.x - x {
                    continue;
                }

                let mut cursor : usize = 0;

                loop {
                    let pattern_symbol = pattern[cursor..]
                        .graphemes(true)
                        .next()
                        .unwrap_or_else(|| {
                            panic!(
                                "Found no char at cursor {} in {}",
                                cursor, &pattern
                            )
                        });

                    let pos_it = Vec2::new(x + cursor, y);

                    let found_symbol: Option<&String> = if let Some(ref cell) = self[&pos_it] {
                        cell.letter.as_option()
                    } else { None };

                    match found_symbol {
                        Some(screen_symbol) => {
                            if pattern_symbol == screen_symbol {
                                cursor += screen_symbol.len();
                            } else {
                                continue 'x;
                            }
                        }
                        None => {
                            if pattern_symbol == " " {
                                cursor += 1;
                            } else {
                                break;
                            }
                        }
                    };

                    if cursor == pattern.len() {
                        break;
                    };
                }

                if cursor == pattern.len() {
                    hits.push(
                        ObservedLine::new(self, Vec2::new(x,y), pattern.len())
                    );
                }
            }
        }
        hits
    }
}

pub trait ObservedPieceInterface {
    fn min(&self) -> Vec2;
    fn max(&self) -> Vec2;
    fn parent(&self) -> &ObservedScreen;

    fn size(&self) -> Vec2 {
        self.max() - self.min()
    }

    fn as_strings(&self) -> Vec<String> {
        let mut v : Vec<String> = vec![];
        for y in self.min().y..self.max().y {
            let mut s = String::new();
            for x in self.min().x..self.max().x {
                match &self.parent()[&Vec2::new(x, y)] {
                    None => s.push(' '),
                    Some(cell) => match &cell.letter {
                        GraphemePart::Begin(lex) => s.push_str(&lex),
                        _ => {}
                    }
                }
            }
            v.push(s);
        }
        v
    }
}

pub struct ObservedPiece<'a> {
    min : Vec2,
    max : Vec2,
    parent : &'a ObservedScreen
}


impl <'a> ObservedPiece<'a> {
    fn new(parent : &'a ObservedScreen, min : Vec2, max : Vec2) -> Self {
        ObservedPiece {
            min,
            max,
            parent
        }
    }
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

pub struct ObservedLine<'a> {
    line_start : Vec2,
    line_len : usize,
    parent : &'a ObservedScreen
}

impl <'a> ObservedLine<'a> {
    fn new(parent : &'a ObservedScreen, line_start : Vec2, line_len : usize) -> Self {
        ObservedLine {
            parent,
            line_start,
            line_len
        }
    }
}

impl <'a> ObservedPieceInterface for ObservedLine<'a> {
    fn min(&self) -> Vec2 {
        self.line_start
    }

    fn max(&self) -> Vec2 {
        self.line_start + Vec2::new(self.line_len, 1)
    }

    fn parent(&self) -> &ObservedScreen {
        self.parent
    }
}


impl <'a> ToString for ObservedLine<'a> {
    fn to_string(&self) -> String {
        self.as_strings().remove(0)
    }
}


impl Index<&Vec2> for ObservedPieceInterface {
    type Output = Option<ObservedCell>;

    fn index(&self, index: &Vec2) -> &Self::Output {
        assert!(self.min().x >= index.x);
        assert!(self.max().x < index.x);
        assert!(self.min().y >= index.y);
        assert!(self.max().y < index.y);

        &self.parent()[&(*index + self.min())]
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

#[cfg(test)]
mod tests {
    use super::*;
    use backend::puppet::DEFAULT_OBSERVED_STYLE;

    /// Expecting fake_screen to be square, # will be replaced with blank.
    fn get_observed_screen(fake_screen : &Vec<&str>) -> ObservedScreen {
        let observed_style : Rc<ObservedStyle> = Rc::new(DEFAULT_OBSERVED_STYLE.clone());

        let height = fake_screen.len();
        let width = fake_screen[0].len();
        let size = Vec2::new(width, height);

        let mut os = ObservedScreen::new(size);

        for y in 0..fake_screen.len() {
            for x in 0..width {
                let letter = fake_screen[y][x..].graphemes(true).next().unwrap().to_owned();
                let idx = os.flatten_index(&Vec2::new(x,y));
                os.contents[idx] = if letter == "#" {
                    None
                } else {
                    Some(ObservedCell::new(
                        Vec2::new(x, y),
                        observed_style.clone(),
                        Some(letter)
                    ))
                };
            }
        }

        os
    }

    #[test]
    fn test_test() {
        let fake_screen : Vec<&'static str> = vec![
            "..hello***",
            "!!##$$$$$*",
            ".hello^^^^",
        ];

        let os = get_observed_screen(&fake_screen);

        assert_eq!(os[&Vec2::new(0,0)].as_ref().unwrap().letter.as_option(), Some(&".".to_owned()));
        assert_eq!(os[&Vec2::new(2,1)], None);
    }

    #[test]
    fn find_occurrences_no_blanks() {
        let fake_screen : Vec<&'static str> = vec![
            "..hello***",
            "!!##$$$$$*",
            ".hello^^^^",
        ];

        let os = get_observed_screen(&fake_screen);

        let hits = os.find_occurences("hello");

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].size(), Vec2::new(5, 1));
        assert_eq!(hits[1].size(), Vec2::new(5, 1));

        assert_eq!(hits[0].to_string(), "hello");
        assert_eq!(hits[1].to_string(), "hello");

        assert_eq!(hits[0].min(), Vec2::new(2, 0));
        assert_eq!(hits[0].max(), Vec2::new(7, 1));

        assert_eq!(hits[1].min(), Vec2::new(1, 2));
        assert_eq!(hits[1].max(), Vec2::new(6, 3));
    }

    #[test]
    fn find_occurrences_some_blanks() {
        let fake_screen : Vec<&'static str> = vec![
            "__hello world_",
            "hello!world___",
            "___hello#world",
        ];

        let os = get_observed_screen(&fake_screen);

        let hits = os.find_occurences("hello world");

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].size(), Vec2::new(11, 1));
        assert_eq!(hits[1].size(), Vec2::new(11, 1));

        assert_eq!(hits[0].to_string(), "hello world");
        assert_eq!(hits[1].to_string(), "hello world");

        assert_eq!(hits[0].min(), Vec2::new(2, 0));
        assert_eq!(hits[0].max(), Vec2::new(13, 1));

        assert_eq!(hits[1].min(), Vec2::new(3, 2));
        assert_eq!(hits[1].max(), Vec2::new(14, 3));
    }
}
