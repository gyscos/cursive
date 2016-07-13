use XY;
use view::View;
use view::SizeCache;
use vec::Vec2;
use printer::Printer;
use orientation::Orientation;
use event::{Event, EventResult, Key};

use std::cmp::min;

/// Arranges its children linearly according to its orientation.
pub struct LinearLayout {
    children: Vec<Child>,
    orientation: Orientation,
    focus: usize,

    cache: Option<XY<SizeCache>>,
}

struct Child {
    view: Box<View>,
    size: Vec2,
    weight: usize,
}

impl Child {
    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
        self.size = self.view.get_min_size(req);
        self.size
    }
}

impl LinearLayout {
    /// Creates a new layout with the given orientation.
    pub fn new(orientation: Orientation) -> Self {
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
    pub fn child<V: View + 'static>(mut self, view: V) -> Self {
        self.children.push(Child {
            view: Box::new(view),
            size: Vec2::zero(),
            weight: 0,
        });
        self.invalidate();

        self
    }

    // Invalidate the view, to request a layout next time
    fn invalidate(&mut self) {
        self.cache = None;
    }

    /// Creates a new vertical layout.
    pub fn vertical() -> Self {
        LinearLayout::new(Orientation::Vertical)
    }

    /// Creates a new horizontal layout.
    pub fn horizontal() -> Self {
        LinearLayout::new(Orientation::Horizontal)
    }

    // If the cache can be used, return the cached size.
    // Otherwise, return None.
    fn get_cache(&self, req: Vec2) -> Option<Vec2> {
        match self.cache {
            None => None,
            Some(ref cache) => {
                // Is our cache even valid?
                // Also, is any child invalidating the layout?
                if cache.x.accept(req.x) && cache.y.accept(req.y) &&
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
            .map(|c| &*c.view)
            .any(View::needs_relayout)
    }
}

/// Returns the index of the maximum element.
/// WTF isn't it part of standard library??
fn find_max(list: &[usize]) -> usize {
    let mut max_value = 0;
    let mut max = 0;
    for (i, &x) in list.iter().enumerate() {
        if x > max_value {
            max_value = x;
            max = i;
        }
    }
    max
}

/// Given a total number of points and a list of weights,
/// try to share the points according to the weight,
/// rounding properly and conserving the sum of points.
fn share(total: usize, weights: Vec<usize>) -> Vec<usize> {
    // It first give a base value to everyone, which is their truncated share.
    // Then, it gives the rest to the most deserving.
    if weights.is_empty() {
        return Vec::new();
    }

    let sum_weight = weights.iter().fold(0, |a, b| a + b);
    if sum_weight == 0 {
        return (0..weights.len()).map(|_| 0).collect();
    }

    let mut base = Vec::with_capacity(weights.len());
    let mut rest = Vec::with_capacity(weights.len());
    let mut extra = total;

    for weight in &weights {
        let b = total * weight / sum_weight;
        extra -= b;
        base.push(b);
        rest.push(total * weight - b * sum_weight);
    }

    // TODO: better to sort (base,rest) as one array and pick the extra first.
    for _ in 0..extra {
        let i = find_max(&rest);
        rest[i] = 0;
        base[i] += 1;
    }

    base
}

impl View for LinearLayout {
    fn draw(&mut self, printer: &Printer) {
        // Use pre-computed sizes
        let mut offset = Vec2::zero();
        for (i, child) in self.children.iter_mut().enumerate() {
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
            self.get_min_size(size);
        }

        for child in &mut self.children {
            // println_stderr!("Child size: {:?}", child.size);
            child.view.layout(child.size);
        }

        /*

        // Need to compute things again...
        self.get_min_size(size);

        let min_sizes: Vec<Vec2> = self.children
            .iter_mut()
            .map(|child| Vec2::min(size, child.view.get_min_size(size)))
            .collect();
        let min_size = self.orientation.stack(min_sizes.iter());

        // Emulate 'non-strict inequality' on integers
        // (default comparison on Vec2 is strict,
        // and (0,1).cmp((1,1)) is undefined)
        if !(min_size < size + (1, 1)) {
            // Error! Not enough space! Emergency procedures!
            return;
        }

        // Now share this extra space among everyone

        let extras = {
            let extra = size - min_size;
            let space = self.orientation.get(&extra);
            share(space,
                  self.children.iter().map(|child| child.weight).collect())
        };


        for (child, (child_size, extra)) in self.children
            .iter_mut()
            .zip(min_sizes.iter().zip(extras.iter())) {
            let mut child_size = *child_size;
            *self.orientation.get_ref(&mut child_size) += *extra;
            *self.orientation.swap().get_ref(&mut child_size) =
                self.orientation.swap().get(&size);
            child.size = child_size;
            child.view.layout(child_size);
        }
        */
    }

    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
        // Did anything change since last time?
        if let Some(size) = self.get_cache(req) {
            return size;
        }

        // First, make a naive scenario: everything will work fine.
        let sizes: Vec<Vec2> = self.children
            .iter_mut()
            .map(|c| c.get_min_size(req))
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

        // Ok, so maybe it didn't.
        // Budget cuts, everyone.
        let budget_req = req.with(self.orientation, 1);
        // println_stderr!("Budget req: {:?}", budget_req);

        let min_sizes: Vec<Vec2> = self.children
            .iter_mut()
            .map(|c| c.get_min_size(budget_req))
            .collect();
        let desperate = self.orientation.stack(min_sizes.iter());
        // println_stderr!("Min sizes: {:?}", min_sizes);
        // println_stderr!("Desperate: {:?}", desperate);

        // I really hope it fits this time...
        if !desperate.fits_in(req) {
            // Just give up...
            // println_stderr!("Seriously? {:?} > {:?}???", desperate, req);
            self.cache = Some(SizeCache::build(desperate, req));
            return desperate;
        }

        // This here is how much we're generously offered
        let mut available = self.orientation.get(&(req - desperate));
        // println_stderr!("Available: {:?}", available);

        // Here, we have to make a compromise between the ideal
        // and the desperate solutions.
        let mut overweight: Vec<(usize, usize)> = sizes.iter()
            .map(|v| self.orientation.get(v))
            .zip(min_sizes.iter().map(|v| self.orientation.get(v)))
            .map(|(a, b)| a - b)
            .enumerate()
            .collect();
        // println_stderr!("Overweight: {:?}", overweight);

        // So... distribute `available` to reduce the overweight...
        // TODO: use child weight in the distribution...
        overweight.sort_by_key(|&(_, weight)| weight);
        let mut allocations = vec![0; overweight.len()];

        for (i, &(j, weight)) in overweight.iter().enumerate() {
            let remaining = overweight.len() - i;
            let budget = available / remaining;
            let spent = min(budget, weight);
            allocations[j] = spent;
            available -= spent;
        }
        // println_stderr!("Allocations: {:?}", allocations);

        // Final lengths are the minimum ones + allocations
        let final_lengths: Vec<Vec2> = min_sizes.iter()
            .map(|v| self.orientation.get(v))
            .zip(allocations.iter())
            .map(|(a, b)| a + b)
            .map(|l| req.with(self.orientation, l))
            .collect();
        // println_stderr!("Final sizes: {:?}", final_lengths);

        let final_sizes: Vec<Vec2> = self.children
            .iter_mut()
            .enumerate()
            .map(|(i, c)| {
                c.get_min_size(final_lengths[i])
            })
            .collect();
        // println_stderr!("Final sizes2: {:?}", final_sizes);


        let compromise = self.orientation.stack(final_sizes.iter());
        self.cache = Some(SizeCache::build(compromise, req));

        compromise
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match self.children[self.focus].view.on_event(event) {
            EventResult::Ignored => {
                match event {
                    Event::Key(Key::Tab) if self.focus > 0 => {
                        self.focus -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Shift(Key::Tab) if self.focus + 1 <
                                              self.children.len() => {
                        self.focus += 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Left) if self.orientation ==
                                             Orientation::Horizontal &&
                                             self.focus > 0 => {
                        self.focus -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Up) if self.orientation ==
                                           Orientation::Vertical &&
                                           self.focus > 0 => {
                        self.focus -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Right) if self.orientation ==
                                              Orientation::Horizontal &&
                                              self.focus + 1 <
                                              self.children.len() => {
                        self.focus += 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::Down) if self.orientation ==
                                             Orientation::Vertical &&
                                             self.focus + 1 <
                                             self.children.len() => {
                        self.focus += 1;
                        EventResult::Consumed(None)
                    }
                    _ => EventResult::Ignored,
                }
            }
            res => res,
        }
    }
}
