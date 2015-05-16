use ::Cursive;
use view::{View,ViewPath,SizeRequest,DimensionRequest};
use vec2::{Vec2};
use printer::Printer;

enum Focus {
    Content,
    Button(usize),
    Nothing,
}

pub struct Dialog {
    content: Box<View>,
    buttons: Vec<Box<View>>,
    focus: Focus,
}

impl Dialog {
    pub fn new<V: View + 'static>(view: V) -> Self {
        Dialog {
            content: Box::new(view),
            buttons: Vec::new(),
            focus: Focus::Nothing,
        }
    }

    pub fn button<'a, F>(self, label: &'a str, cb: F) -> Self
        where F: Fn(&mut Cursive, &ViewPath)
    {
        self
    }
}

fn offset_request(request: DimensionRequest, offset: i32) -> DimensionRequest {
    match request {
        DimensionRequest::Fixed(w) => DimensionRequest::Fixed((w as i32 + offset) as u32),
        DimensionRequest::AtMost(w) => DimensionRequest::AtMost((w as i32 + offset) as u32),
        DimensionRequest::Unknown => DimensionRequest::Unknown,
    }
}

impl View for Dialog {
    fn draw(&self, printer: &Printer) {
        
    }

    fn get_min_size(&self, size: SizeRequest) -> Vec2 {
        let content_req = SizeRequest {
            w: offset_request(size.w, -2),
            h: offset_request(size.h, -2),
        };
        let content_size = self.content.get_min_size(content_req);

        content_size + (2u32,2u32)
    }
}
