use backend::puppet::observed::ObservedScreen;
use view::View;
use Printer;
use Vec2;
use backend::puppet::observed::ObservedCell;
use theme::ColorStyle;
use theme::ColorType;
use backend::puppet::observed::GraphemePart;

pub struct ObservedScreenView {
    screen : ObservedScreen,
}

impl ObservedScreenView {
    pub fn new(obs : ObservedScreen) -> Self {
        ObservedScreenView { screen : obs }
    }
}

impl View for ObservedScreenView {
    fn draw(&self, printer: &Printer) {

        for x in 0..self.screen.size().x {
            for y in 0..self.screen.size().y {
                let pos = Vec2::new(x,y);
                let cell : &ObservedCell = self.screen[&pos].as_ref().unwrap();

                if cell.letter.is_continuation() {
                    continue;
                }

                printer.with_effects(cell.style.effects, | printer | {
                    let color_style = ColorStyle {
                        front: ColorType::Color(cell.style.colors.front),
                        back: ColorType::Color(cell.style.colors.back),
                    };

                    printer.print(pos, cell.letter.unwrap());
                })
            }
        }
    }

    fn required_size(&mut self, _: Vec2) -> Vec2 {
        self.screen.size()
    }

}