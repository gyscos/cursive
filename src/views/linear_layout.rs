use Printer;
use With;
use XY;
use direction;
use event::{Event, EventResult, Key};

use std::any::Any;
use std::cmp::min;
use vec::Vec2;
use view::{Selector, SizeCache};
use view::View;

/// Arranges its children linearly according to its orientation.
pub struct LinearLayout {
    children: Vec<Child>,
    orientation: direction::Orientation,
    focus: usize,

    cache: Option<XY<SizeCache>>,
}

struct Child {
    view: Box<View>,
    size: Vec2,
    weight: usize,
}

impl Child {
    // Compute and caches the required size.
    fn required_size(&mut self, req: Vec2) -> Vec2 {
        self.size = self.view.required_size(req);
        self.size
    }

    fn as_view(&self) -> &View {
        &*self.view
    }
}

fn cap<'a, I: Iterator<Item = &'a mut usize>>(iter: I, max: usize) {
    let mut available = max;
    for item in iter {
        if *item > available {
            *item = available;
        }

        available -= *item;
    }
}

impl LinearLayout {
    /// Creates a new layout with the given orientation.
    pub fn new(orientation: direction::Orientation) -> Self {
        LinearLayout {
            children: Vec::new(),
            orientation: orientation,
            focus: 0,
            cache: None,
        }
    }

    /// Modifies the weight of the last child added.
    ///
    /// It is an error to call this before adding a child (and it will panic).
    pub fn weight(mut self, weight: usize) -> Self {
        self.children.last_mut().unwrap().weight = weight;

        self
    }

    /// Adds a child to the layout.
    ///
    /// Chainable variant.
    pub fn child<V: View + 'static>(self, view: V) -> Self {
        self.with(|s| s.add_child(view))
    }

    /// Adds a child to the layout.
    pub fn add_child<V: View + 'static>(&mut self, view: V) {
        self.children.push(Child {
            view: Box::new(view),
            size: Vec2::zero(),
            weight: 0,
        });
        self.invalidate();
    }

    // Invalidate the view, to request a layout next time
    fn invalidate(&mut self) {
        self.cache = None;
    }

    /// Creates a new vertical layout.
    pub fn vertical() -> Self {
        LinearLayout::new(direction::Orientation::Vertical)
    }

    /// Creates a new horizontal layout.
    pub fn horizontal() -> Self {
        LinearLayout::new(direction::Orientation::Horizontal)
    }

    // If the cache can be used, return the cached size.
    // Otherwise, return None.
    fn get_cache(&self, req: Vec2) -> Option<Vec2> {
        match self.cache {
            None => None,
            Some(ref cache) => {
                // Is our cache even valid?
                // Also, is any child invalidating the layout?
                if cache.zip_map(req, SizeCache::accept).both() &&
                   self.children_are_sleeping() {
                    Some(cache.map(|s| s.value))
                } else {
                    None
                }
            }
        }
    }

    fn children_are_sleeping(&self) -> bool {
        !self.children
            .iter()
            .map(Child::as_view)
            .any(View::needs_relayout)
    }

    /// Returns a cyclic mutable iterator starting with the child in focus
    fn iter_mut<'a>(&'a mut self, from_focus: bool,
                    source: direction::Relative)
                    -> Box<Iterator<Item = (usize, &mut Child)> + 'a> {

        match source {
            direction::Relative::Front => {
                let start = if from_focus { self.focus } else { 0 };

                Box::new(self.children.iter_mut().enumerate().skip(start))
            }
            direction::Relative::Back => {
                let end = if from_focus {
                    self.focus + 1
                } else {
                    self.children.len()
                };
                Box::new(self.children[..end].iter_mut().enumerate().rev())
            }
        }
    }

    fn move_focus(&mut self, source: direction::Direction) -> EventResult {

        let i = if let Some(i) = source.relative(self.orientation)
            .and_then(|rel| {
                // The iterator starts at the focused element.
                // We don't want that one.
                self.iter_mut(true, rel)
                    .skip(1)
                    .filter_map(|p| try_focus(p, source))
                    .next()
            }) {
            i
        } else {
            return EventResult::Ignored;
        };
        self.focus = i;
        EventResult::Consumed(None)
    }
}

fn try_focus((i, child): (usize, &mut Child), source: direction::Direction)
             -> Option<usize> {
    if child.view.take_focus(source) {
        Some(i)
    } else {
        None
    }
}

impl View for LinearLayout {
    fn draw(&self, printer: &Printer) {
        // Use pre-computed sizes
        let mut offset = Vec2::zero();
        for (i, child) in self.children.iter().enumerate() {
            let printer =
                &printer.sub_printer(offset, child.size, i == self.focus);
            child.view.draw(printer);

            // On the axis given by the orientation,
            // add the child size to the offset.
            *self.orientation.get_ref(&mut offset) += self.orientation
                .get(&child.size);
        }
    }

    fn needs_relayout(&self) -> bool {
        if self.cache.is_none() {
            return true;
        }

        !self.children_are_sleeping()
    }

    fn layout(&mut self, size: Vec2) {
        // If we can get away without breaking a sweat, you can bet we will.
        if self.get_cache(size).is_none() {
            self.required_size(size);
        }

        let o = self.orientation;

        for child in &mut self.children {
            // Every item has the same size orthogonal to the layout
            child.size.set_axis_from(o.swap(), &size);
            child.view.layout(size.with_axis_from(o, &child.size));
        }
    }

    fn required_size(&mut self, req: Vec2) -> Vec2 {
        // Did anything change since last time?
        if let Some(size) = self.get_cache(req) {
            return size;
        }

        // First, make a naive scenario: everything will work fine.
        let sizes: Vec<Vec2> = self.children
            .iter_mut()
            .map(|c| c.required_size(req))
            .collect();
        // println_stderr!("Ideal sizes: {:?}", sizes);
        let ideal = self.orientation.stack(sizes.iter());
        // println_stderr!("Ideal result: {:?}", ideal);


        // Does it fit?
        if ideal.fits_in(req) {
            // Champagne!
            self.cache = Some(SizeCache::build(ideal, req));
            return ideal;
        }

        // Ok, so maybe it didn't. Budget cuts, everyone.
        // Let's pretend we have almost no space in this direction.
        let budget_req = req.with_axis(self.orientation, 1);
        // println_stderr!("Budget req: {:?}", budget_req);

        // See how they like it that way
        let min_sizes: Vec<Vec2> = self.children
            .iter_mut()
            .map(|c| c.required_size(budget_req))
            .collect();
        let desperate = self.orientation.stack(min_sizes.iter());
        // println_stderr!("Min sizes: {:?}", min_sizes);
        // println_stderr!("Desperate: {:?}", desperate);

        // This is the lowest we'll ever go. It better fit at least.
        let orientation = self.orientation;
        if !desperate.fits_in(req) {
            // Just give up...
            // TODO: hard-cut
            cap(self.children
                    .iter_mut()
                    .map(|c| c.size.get_mut(orientation)),
                *req.get(self.orientation));

            // TODO: print some error message or something
            // println_stderr!("Seriously? {:?} > {:?}???", desperate, req);
            // self.cache = Some(SizeCache::build(desperate, req));
            self.cache = None;
            return desperate;
        }

        // This here is how much we're generously offered
        // (We just checked that req >= desperate, so the substraction is safe
        let mut available = self.orientation.get(&(req - desperate));
        // println_stderr!("Available: {:?}", available);

        // Here, we have to make a compromise between the ideal
        // and the desperate solutions.
        let mut overweight: Vec<(usize, usize)> = sizes.iter()
            .map(|v| self.orientation.get(v))
            .zip(min_sizes.iter().map(|v| self.orientation.get(v)))
            .map(|(a, b)| if a > b { a - b } else { 0 })
            .enumerate()
            .collect();
        // println_stderr!("Overweight: {:?}", overweight);

        // So... distribute `available` to reduce the overweight...
        // TODO: use child weight in the distribution...

        // We'll give everyone his share of what we have left,
        // starting with those who ask the least.
        overweight.sort_by_key(|&(_, weight)| weight);
        let mut allocations = vec![0; overweight.len()];

        for (i, &(j, weight)) in overweight.iter().enumerate() {
            // This is the number of people we still have to feed.
            let remaining = overweight.len() - i;
            // How much we can spare on each one
            let budget = available / remaining;
            // Maybe he doesn't even need that much?
            let spent = min(budget, weight);
            allocations[j] = spent;
            available -= spent;
        }
        // println_stderr!("Allocations: {:?}", allocations);

        // Final lengths are the minimum ones + generous allocations
        let final_lengths: Vec<Vec2> = min_sizes.iter()
            .map(|v| self.orientation.get(v))
            .zip(allocations.iter())
            .map(|(a, b)| a + b)
            .map(|l| req.with_axis(self.orientation, l))
            .collect();
        // println_stderr!("Final sizes: {:?}", final_lengths);

        // Let's ask everyone one last time. Everyone should be happy.
        // (But they may ask more on the other axis.)
        let final_sizes: Vec<Vec2> = self.children
            .iter_mut()
            .enumerate()
            .map(|(i, c)| c.required_size(final_lengths[i]))
            .collect();
        // println_stderr!("Final sizes2: {:?}", final_sizes);

        // Let's stack everything to see what it looks like.
        let compromise = self.orientation.stack(final_sizes.iter());

        // Phew, that was a lot of work! I'm not doing it again.
        self.cache = Some(SizeCache::build(compromise, req));

        compromise
    }

    fn take_focus(&mut self, source: direction::Direction) -> bool {
        // In what order will we iterate on the children?
        let rel = source.relative(self.orientation);
        // We activate from_focus only if coming from the "sides".
        let i = if let Some(i) = self.iter_mut(rel.is_none(),
                      rel.unwrap_or(direction::Relative::Front))
            .filter_map(|p| try_focus(p, source))
            .next() {

            // ... we can't update `self.focus` here,
            // because rustc thinks we still borrow `self`.
            // :(
            i
        } else {
            return false;
        };

        self.focus = i;
        true
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match self.children[self.focus].view.on_event(event.clone()) {
            EventResult::Ignored => {
                match event {
                    Event::Shift(Key::Tab) if self.focus > 0 => {
                        self.move_focus(direction::Direction::back())
                    }
                    Event::Key(Key::Tab) if self.focus + 1 <
                                            self.children.len() => {
                        self.move_focus(direction::Direction::front())
                    }
                    Event::Key(Key::Left)
                        if self.orientation ==
                           direction::Orientation::Horizontal &&
                           self.focus > 0 => {
                        self.move_focus(direction::Direction::right())
                    }
                    Event::Key(Key::Up) if self.orientation ==
                                           direction::Orientation::Vertical &&
                                           self.focus > 0 => {
                        self.move_focus(direction::Direction::down())
                    }
                    Event::Key(Key::Right)
                        if self.orientation ==
                           direction::Orientation::Horizontal &&
                           self.focus + 1 <
                           self.children.len() => {
                        self.move_focus(direction::Direction::left())
                    }
                    Event::Key(Key::Down)
                        if self.orientation ==
                           direction::Orientation::Vertical &&
                           self.focus + 1 <
                           self.children.len() => {
                        self.move_focus(direction::Direction::up())
                    }
                    _ => EventResult::Ignored,
                }
            }
            res => res,
        }
    }

    fn find_any<'a>(&mut self, selector: &Selector,
                    mut callback: Box<FnMut(&mut Any) + 'a>) {
        for child in &mut self.children {
            child.view.find_any(selector, Box::new(|any| callback(any)));
        }
    }

    fn focus_view(&mut self, selector: &Selector) -> Result<(), ()> {
        for (i, child) in self.children.iter_mut().enumerate() {
            if child.view.focus_view(selector).is_ok() {
                self.focus = i;
                return Ok(());
            }
        }

        Err(())
    }
}
