use vec::Vec2;
use view::{View,SizeRequest};
use printer::Printer;
use event::EventResult;

pub trait ViewWrapper {
    fn get_view(&self) -> &View;
    fn get_view_mut(&mut self) -> &mut View;

    fn wrap_draw(&self, printer: &Printer, focused: bool) {
        self.get_view().draw(printer, focused);
    }

    fn wrap_get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.get_view().get_min_size(req)
    }

    fn wrap_on_key_event(&mut self, ch: i32) -> EventResult {
        self.get_view_mut().on_key_event(ch)
    }

    fn wrap_layout(&mut self, size: Vec2) {
        self.get_view_mut().layout(size);
    }
}

impl <T: ViewWrapper> View for T {
    fn draw(&self, printer: &Printer, focused: bool) {
        self.wrap_draw(printer, focused);
    }

    fn get_min_size(&self, req: SizeRequest) -> Vec2 {
        self.wrap_get_min_size(req)
    }

    fn on_key_event(&mut self, ch: i32) -> EventResult {
        self.wrap_on_key_event(ch)
    }

    fn layout(&mut self, size: Vec2) {
        self.wrap_layout(size);
    }
}

#[macro_export]
macro_rules! wrap_impl {
    (&self.$v:ident) => {

        fn get_view(&self) -> &View {
            &self.$v
        }

        fn get_view_mut(&mut self) -> &mut View {
            &mut self.$v
        }
    };
    (self.$v:ident) => {

        fn get_view(&self) -> &View {
            &*self.$v
        }

        fn get_view_mut(&mut self) -> &mut View {
            &mut *self.$v
        }
    };
}
