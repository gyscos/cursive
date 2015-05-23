use view::{View,ViewWrapper,SizeRequest};
use printer::Printer;
use vec::Vec2;
use color;

pub struct ShadowView<T: View> {
    pub view: T,
}

impl <T: View> ShadowView<T> {
    pub fn new(view: T) -> Self {
        ShadowView {
            view: view,
        }
    }
}

impl <T: View> ViewWrapper for ShadowView<T> {

    wrap_impl!(&self.view);

    fn wrap_get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.view.get_min_size(req.reduced((2,2))) + (2,2)
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.view.layout(size - (2,2));
    }

    fn wrap_draw(&mut self, printer: &Printer, focused: bool) {

        {
            printer.with_style(color::PRIMARY, |printer| {
                // Draw the view background
                for y in 1..printer.size.y-1 {
                    printer.print_hline((1,y), printer.size.x-2, ' ' as u64);
                }
            });
        }


        self.view.draw(&printer.sub_printer(Vec2::new(1,1), printer.size - (2,2)), focused);


        let h = printer.size.y-1;
        let w = printer.size.x-1;

        printer.with_style(color::SHADOW, |printer| {
            printer.print_hline((2,h), w-1, ' ' as u64);
            printer.print_vline((w,2), h-1, ' ' as u64);
        });
    }
}
