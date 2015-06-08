use view::{View,SizeRequest,DimensionRequest};
use vec::Vec2;
use printer::Printer;
use orientation::Orientation;

struct Child {
    view: Box<View>,
    size: Vec2,
    weight: usize,
}

pub struct LinearLayout {
    children: Vec<Child>,
    orientation: Orientation,
}


impl LinearLayout {
    pub fn new(orientation: Orientation) -> Self {
        LinearLayout {
            children: Vec::new(),
            orientation: orientation,
        }
    }

    pub fn weight(mut self, weight: usize) -> Self {
        self.children.last_mut().unwrap().weight = weight;

        self
    }

    pub fn child<V: View + 'static>(mut self, view: V) -> Self {
        self.children.push(Child {
            view: Box::new(view),
            size: Vec2::zero(),
            weight: 0,
        });

        self
    }

    pub fn vertical() -> Self {
        LinearLayout::new(Orientation::Vertical)
    }
    pub fn horizontal() -> Self {
        LinearLayout::new(Orientation::Horizontal)
    }
}

fn find_max(list: &Vec<usize>) -> usize {
    let mut max_value = 0;
    let mut max = 0;
    for (i,&x) in list.iter().enumerate() {
        if x > max_value {
            max_value = x;
            max = i;
        }
    }
    max
}

fn share(total: usize, weights: Vec<usize>) -> Vec<usize> {
    if weights.len() == 0 { return Vec::new(); }

    let sum_weight = weights.iter().fold(0,|a,b| a+b);
    if sum_weight == 0 {
        return (0..weights.len()).map(|_| 0).collect();
    }

    let mut base = Vec::with_capacity(weights.len());
    let mut rest = Vec::with_capacity(weights.len());
    let mut extra = total;

    for weight in weights.iter() {
        let b = total * weight / sum_weight;
        extra -= b;
        base.push(b);
        rest.push(total * weight - b*sum_weight);
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
        for child in self.children.iter_mut() {
            println!("Drawing at {:?} for {:?}", offset, child.size);
            child.view.draw(&printer.sub_printer(offset, child.size, true));

            *self.orientation.get_ref(&mut offset) += self.orientation.get(&child.size);
        }
    }

    fn layout(&mut self, size: Vec2) {
        println!("LAYOUT!!");
        // Compute the very minimal required size
        let req = SizeRequest{
            w: DimensionRequest::AtMost(size.x),
            h: DimensionRequest::AtMost(size.y),
        };
        let min_sizes: Vec<Vec2> = self.children.iter().map(|child| child.view.get_min_size(req)).collect();
        let min_size = self.orientation.stack(min_sizes.iter());

        // Emulate 'non-strict inequality' on integers
        // (default comparison on Vec2 is strict)
        if !(min_size < size+(1,1)) {
            // Error! Not enough space! Emergency procedures!
            println!("Min: {:?}, size: {:?}", min_size, size);
            return
        }

        // Now share this extra space among everyone

        let extras = {
            let extra = size - min_size;
            let space = self.orientation.get(&extra);
            share(space, self.children.iter().map(|child| child.weight).collect())
        };

        println!("Sizes: {}, {}, {}", self.children.len(), min_sizes.len(), extras.len());

        for (child,(child_size,extra)) in self.children.iter_mut().zip(min_sizes.iter().zip(extras.iter())) {
            println!("Child size: {:?}", child_size);
            let mut child_size = *child_size;
            *self.orientation.get_ref(&mut child_size) += *extra;
            *self.orientation.swap().get_ref(&mut child_size) = self.orientation.swap().get(&size);
            child.size = child_size;
            child.view.layout(child_size);
        }
    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        // First, make a naive scenario: everything will work fine.
        let sizes: Vec<Vec2> = self.children.iter().map(|view| view.view.get_min_size(req)).collect();
        println!("Views size: {:?}", sizes);
        self.orientation.stack(sizes.iter())


        // Did it work? Champagne!


        // Ok, so maybe it didn't.
        // Last chance: did someone lie about his needs?
        // Could we squash him a little?

        // Find out who's fluid, if any.
    }
}
