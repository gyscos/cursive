//! Base elements required to build views.
//!
//! Views are the main building blocks of your UI.
//!
//! A view can delegate part or all of its responsabilities to child views,
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

#[macro_use]
mod view_wrapper;

// Essentials components
mod any;
mod finder;
mod margins;
mod position;
mod size_cache;
mod size_constraint;
mod view_path;
mod view_trait;

// Helper bases
mod boxable;
mod identifiable;
#[cfg(feature = "unstable_scroll")]
pub mod scroll;

#[cfg(not(feature = "unstable_scroll"))]
#[allow(dead_code)]
pub(crate) mod scroll;

mod scroll_base;
mod scrollable;

mod into_boxed_view;

pub use self::any::AnyView;
pub use self::boxable::Boxable;
pub use self::finder::{Finder, Selector};
pub use self::identifiable::Identifiable;
pub use self::into_boxed_view::IntoBoxedView;
pub use self::margins::Margins;
pub use self::position::{Offset, Position};
pub use self::scroll::ScrollStrategy;
pub use self::scroll_base::ScrollBase;
pub use self::scrollable::Scrollable;
pub use self::size_cache::SizeCache;
pub use self::size_constraint::SizeConstraint;
pub use self::view_path::ViewPath;
pub use self::view_trait::View;
pub use self::view_wrapper::ViewWrapper;
