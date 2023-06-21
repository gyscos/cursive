//! Base elements required to build views.
//!
//! Views are the main building blocks of your UI.
//!
//! A view can delegate part or all of its responsibilities to child views,
//! forming a view tree. The root of this tree is a `StackView` handled
//! directly by the `Cursive` element.
//!
//! # Layout
//!
//! The layout phase is when the size and location of each view is computed.
//!
//! Each view is given an area of the screen by the `View::layout()` method.
//! With this, the view is free to plan its content, including calling
//! `View::layout()` on its own children.
//!
//! In order to determine how much space should be given each child, parents
//! can use `View::required_size()` on them.
//!
//! ### Contracts
//!
//! When building new Views, you should respect these contracts:
//!
//! * By default, `View::layout()` should be called before any call to
//!   `View::draw()` with the same available size. The only exceptions is
//!   when both following conditions are met:
//!     * The available size has not changed since the last call to
//!       `View::layout()`
//!     * `View::needs_relayout()` returns `false`
//!
//!   In this case, it is safe to omit the call to `View::layout()`.
//!
//! * The value returned by `required_size` should be an actually viable size,
//!   no matter what the request is. This means calling `View::layout()` with
//!   a size returned by `required_size` is **never** an error.
//!
//! # Focus
//!
//! In most layouts, a single view is active at any given time. This focus may
//! change in response to events (for example presing the Tab key often moves
//! to the next item).
//!
//! This focus system involves two mechanics:
//! * Individual views can decide whether they can be focused or not, through
//!   the `View::take_focus()` method. For example, unless disabled, a `Button`
//!   would accept focus (and return `true` from `take_focus()`), but a simple
//!   `TextView` or a divider would not (they would return `false`).
//! * View groups like `LinearLayout` listen to events ignored by their
//!   children, and change their focus accordingly. For example, if the `Tab`
//!   key is pressed but the currently focused child of the `LinearLayout`
//!   ignores this event, then the `LinearLayout` will attempt to focus the
//!   next child. If no child accept the focus, then it will ignore the event
//!   as well.
//!
//! # Scrolling
//!
//! Most views do not scroll by themselves; instead, they should be wrapped in
//! a `ScrollView` to enable scrolling. The `ScrollView` will pretend that the
//! wrapped view has been given a large enough area to fit entirely, but in
//! reality only a part of that will be visible.
//!
//! The wrapped view can ignore this and just draw itself as usual: the
//! `Printer` will transparently translate the calls, and print commands
//! outside of the visible area will simply be ignored.
//!
//! In some cases however it may be interesting for the nested view to know
//! about this, maybe to avoid computing parts of the view that are not
//! visible. `Printer::output_size` and `Printer::content_offset` can be used
//! to find out what part of the view should actually be printed.
//!
//! ## Important Area
//!
//! Sometimes, the wrapped view needs to communicate back to the `ScrollView`
//! what part of the view is really important and should be kept visible.
//!
//! For example, imagine a vertical list of buttons. When the user selects the
//! next button, we want to scroll down a bit so the button becomes visible if
//! it wasn't. To achieve this, the vertical `LinearLayout` communicates its
//! "important area" (the currently active button) to the parent `ScrollView`,
//! and the `ScrollView` makes sure that this area stays in view.

#[macro_use]
mod view_wrapper;

// Essentials components
mod any;
mod finder;
mod margins;
mod position;
mod size_cache;
mod size_constraint;
mod view_trait;

#[macro_use]
pub mod scroll;

// Helper bases
mod into_boxed_view;
mod nameable;
mod resizable;
mod scrollable;

// That one is deprecated
mod scroll_base;

pub use self::any::AnyView;
pub use self::finder::{Finder, Selector};
pub use self::into_boxed_view::IntoBoxedView;
pub use self::margins::Margins;
pub use self::nameable::Nameable;
pub use self::position::{Offset, Position};
pub use self::resizable::Resizable;
pub use self::scroll::ScrollStrategy;
#[allow(deprecated)]
pub use self::scroll_base::ScrollBase;
pub use self::scrollable::Scrollable;
pub use self::size_cache::SizeCache;
pub use self::size_constraint::SizeConstraint;
pub use self::view_trait::{CannotFocus, View, ViewNotFound};
pub use self::view_wrapper::ViewWrapper;
