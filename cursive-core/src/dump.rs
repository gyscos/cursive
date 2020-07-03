use crate::{theme::Theme, views, Cursive};
use crossbeam_channel::{Receiver, Sender};
use std::any::Any;
use std::num::NonZeroU32;

/// Represents a dump of everything from a `Cursive` instance.
///
/// See [`Cursive::dump()`](../cursive.html#method.dump)
pub struct Dump {
    pub(crate) cb_sink: Sender<Box<dyn FnOnce(&mut Cursive) + Send>>,
    pub(crate) cb_source: Receiver<Box<dyn FnOnce(&mut Cursive) + Send>>,

    pub(crate) fps: Option<NonZeroU32>,

    pub(crate) menubar: views::Menubar,
    pub(crate) root_view:
        views::OnEventView<views::ScreensView<views::StackView>>,

    pub(crate) theme: Theme,

    pub(crate) user_data: Box<dyn Any>,
}
