use super::View;

pub trait ToView {
    fn to_view(self) -> Box<View>;
}

impl<'a> ToView for &'a str {
    fn to_view(self) -> Box<View> {
        Box::new(TextView::new(self))
    }
}

