use view::View;
use vec::Vec2;
use printer::Printer;
use orientation::Orientation;
use event::{Event, EventResult, Key};

/// Arranges its children linearly according to its orientation.
pub struct LinearLayout {
    children: Vec<Child>,
    orientation: Orientation,
    focus: usize,
}

struct Child {
    view: Box<View>,
    size: Vec2,
    weight: usize,
}

impl LinearLayout {
    /// Creates a new layout with the given orientation.
    pub fn new(orientation: Orientation) -> Self {
        LinearLayout {
            children: Vec::new(),
            orientation: orientation,
            focus: 0,
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

        self
    }

    /// Creates a new vertical layout.
    pub fn vertical() -> Self {
        LinearLayout::new(Orientation::Vertical)
    }

    /// Creates a new horizontal layout.
    pub fn horizontal() -> Self {
        LinearLayout::new(Orientation::Horizontal)
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
            child.view.draw(&printer.sub_printer(offset, child.size, i == self.focus));

            // On the axis given by the orientation,
            // add the child size to the offset.
            *self.orientation.get_ref(&mut offset) += self.orientation
                .get(&child.size);
        }
    }

    fn layout(&mut self, size: Vec2) {
        // Compute the very minimal required size
        // Look how mean we are: we offer the whole size to every child.
        // As if they could get it all.
        let min_sizes: Vec<Vec2> = self.children
            .iter_mut()
            .map(|child| Vec2::min(size, child.view.get_min_size(size)))
            .collect();
        let min_size = self.orientation.stack(min_sizes.iter());

        // Emulate 'non-strict inequality' on integers
        // (default comparison on Vec2 is strict, and (0,1).cmp((1,1)) is undefined)
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
    }

    fn get_min_size(&mut self, req: Vec2) -> Vec2 {
        // First, make a naive scenario: everything will work fine.
        let sizes: Vec<Vec2> = self.children
            .iter_mut()
            .map(|view| view.view.get_min_size(req))
            .collect();
        self.orientation.stack(sizes.iter())


        // Did it work? Champagne!


        // TODO: Ok, so maybe it didn't.
        // Last chance: did someone lie about his needs?
        // Could we squash him a little?
        // (Maybe he'll just scroll and it'll be fine?)

        // Find out who's fluid, if any.
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match self.children[self.focus].view.on_event(event) {
            EventResult::Ignored => {
                match event {
                    Event::Key(Key::Tab) if self.focus > 0 => {
                        self.focus -= 1;
                        EventResult::Consumed(None)
                    }
                    Event::Key(Key::ShiftTab) if self.focus + 1 <
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
