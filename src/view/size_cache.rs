use crate::vec::Vec2;
use crate::XY;

/// Cache around a one-dimensional layout result.
///
/// This is not a View, but something to help you if you create your own Views.
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct SizeCache {
    /// Cached value
    pub value: usize,
    /// `true` if the last size was constrained.
    ///
    /// If unconstrained, any request larger than this value
    /// would return the same size.
    pub constrained: bool,
}

impl SizeCache {
    /// Creates a new sized cache
    pub fn new(value: usize, constrained: bool) -> Self {
        SizeCache { value, constrained }
    }

    /// Returns `true` if `self` is still valid for the given `request`.
    pub fn accept(self, request: usize) -> bool {
        if request < self.value {
            false
        } else if request == self.value {
            true
        } else {
            !self.constrained
        }
    }

    /// Returns the value in the cache.
    pub fn value(self) -> usize {
        self.value
    }

    /// Creates a new bi-dimensional cache.
    ///
    /// It will stay valid for the same request, and compatible ones.
    ///
    /// A compatible request is one where, for each axis, either:
    ///
    /// * the request is equal to the cached size, or
    /// * the request is larger than the cached size and the cache is
    ///   unconstrained
    ///
    /// Notes:
    ///
    /// * `size` must fit inside `req`.
    /// * for each dimension, `constrained = (size == req)`
    pub fn build(size: Vec2, req: Vec2) -> XY<Self> {
        size.zip_map(req, |size, req| SizeCache::new(size, size >= req))
    }
}
