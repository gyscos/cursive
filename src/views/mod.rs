//! Various views to use when creating the layout.

/// A macro to help with creating toggleable views.
///
/// # Examples
///
/// ```
/// struct MyView {
///     enabled: bool,
/// }
///
/// impl MyView {
///     cursive::impl_enabled!(self.enabled);
/// }
///
/// let view = MyView { enabled: true };
/// assert!(view.is_enabled());
/// ```
#[macro_export]
macro_rules! impl_enabled {
    (self.$x:ident) => {

        /// Disables this view.
        ///
        /// A disabled view cannot be selected.
        pub fn disable(&mut self) {
            self.$x = false;
        }

        /// Disables this view.
        ///
        /// Chainable variant.
        pub fn disabled(self) -> Self {
            use $crate::traits::With as _;
            self.with(Self::disable)
        }

        /// Re-enables this view.
        pub fn enable(&mut self) {
            self.$x = true;
        }

        /// Enable or disable this view.
        pub fn set_enabled(&mut self, enabled: bool) {
            self.$x = enabled;
        }

        /// Returns `true` if this view is enabled.
        pub fn is_enabled(&self) -> bool {
            self.$x
        }
    }
}

mod box_view;
mod button;
mod canvas;
mod checkbox;
mod circular_focus;
mod debug_view;
mod dialog;
mod dummy;
mod edit_view;
mod enableable_view;
mod hideable_view;
mod id_view;
mod layer;
mod linear_layout;
mod list_view;
mod menu_popup;
mod menubar;
mod on_event_view;
mod padded_view;
mod panel;
mod progress_bar;
mod radio;
mod scroll_view;
mod select_view;
mod shadow_view;
mod sized_view;
mod slider_view;
mod stack_view;
mod text_area;
mod text_view;
mod tracked_view;
mod view_box;

pub use self::box_view::BoxView;
pub use self::button::Button;
pub use self::canvas::Canvas;
pub use self::checkbox::Checkbox;
pub use self::circular_focus::CircularFocus;
pub use self::debug_view::DebugView;
pub use self::dialog::{Dialog, DialogFocus};
pub use self::dummy::DummyView;
pub use self::edit_view::EditView;
pub use self::enableable_view::EnableableView;
pub use self::hideable_view::HideableView;
pub use self::id_view::{IdView, ViewRef};
pub use self::layer::Layer;
pub use self::linear_layout::LinearLayout;
pub use self::list_view::{ListChild, ListView};
pub use self::menu_popup::MenuPopup;
pub use self::menubar::Menubar;
pub use self::on_event_view::OnEventView;
pub use self::padded_view::PaddedView;
pub use self::panel::Panel;
pub use self::progress_bar::ProgressBar;
pub use self::radio::{RadioButton, RadioGroup};
pub use self::scroll_view::ScrollView;
pub use self::select_view::SelectView;
pub use self::shadow_view::ShadowView;
pub use self::sized_view::SizedView;
pub use self::slider_view::SliderView;
pub use self::stack_view::{LayerPosition, StackView};
pub use self::text_area::TextArea;
pub use self::text_view::{TextContent, TextContentRef, TextView};
pub use self::tracked_view::TrackedView;
pub use self::view_box::ViewBox;
