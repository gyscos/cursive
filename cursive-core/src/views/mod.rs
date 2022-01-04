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
///     cursive_core::impl_enabled!(self.enabled);
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

        /// Enable or disable this view.
        ///
        /// Chainable variant.
        pub fn with_enabled(mut self, is_enabled: bool) -> Self {
            self.set_enabled(is_enabled);
            self
        }

        /// Returns `true` if this view is enabled.
        pub fn is_enabled(&self) -> bool {
            self.$x
        }
    };
}

mod boxed_view;
mod button;
mod canvas;
mod checkbox;
mod circular_focus;
mod debug_view;
mod dialog;
mod dummy;
mod edit_view;
mod enableable_view;
mod fixed_layout;
mod focus_tracker;
mod hideable_view;
mod last_size_view;
mod layer;
mod linear_layout;
mod list_view;
mod menu_popup;
mod menubar;
mod named_view;
mod on_event_view;
mod on_layout_view;
mod padded_view;
mod panel;
mod progress_bar;
mod radio;
mod resized_view;
mod screens_view;
mod scroll_view;
mod select_view;
mod shadow_view;
mod slider_view;
mod stack_view;
mod text_area;
mod text_view;
mod themed_view;
mod tracked_view;

pub use self::{
    boxed_view::BoxedView,
    button::Button,
    canvas::Canvas,
    checkbox::Checkbox,
    circular_focus::CircularFocus,
    debug_view::DebugView,
    dialog::{Dialog, DialogFocus},
    dummy::DummyView,
    edit_view::EditView,
    enableable_view::EnableableView,
    fixed_layout::FixedLayout,
    focus_tracker::FocusTracker,
    hideable_view::HideableView,
    last_size_view::LastSizeView,
    layer::Layer,
    linear_layout::LinearLayout,
    list_view::{ListChild, ListView},
    menu_popup::MenuPopup,
    menubar::Menubar,
    named_view::{NamedView, ViewRef},
    on_event_view::OnEventView,
    on_layout_view::OnLayoutView,
    padded_view::PaddedView,
    panel::Panel,
    progress_bar::ProgressBar,
    radio::{RadioButton, RadioGroup},
    resized_view::ResizedView,
    screens_view::ScreensView,
    scroll_view::ScrollView,
    select_view::SelectView,
    shadow_view::ShadowView,
    slider_view::SliderView,
    stack_view::{LayerPosition, StackView},
    text_area::TextArea,
    text_view::{TextContent, TextContentRef, TextView},
    themed_view::ThemedView,
    tracked_view::TrackedView,
};
