use crate::Vec2;
use crate::XY;

/// Cache around a one-dimensional layout result.
///
/// This is not a View, but something to help you if you create your own Views.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct SizeCache<T = ()> {
    /// Cached value
    pub value: usize,
    /// `true` if the last size was constrained.
    ///
    /// If unconstrained, any request larger than this value
    /// would return the same size.
    pub constrained: bool,

    /// Extra field.
    pub extra: T,
}

impl SizeCache<()> {
    /// Creates a new sized cache
    pub fn new(value: usize, constrained: bool) -> Self {
        SizeCache {
            value,
            constrained,
            extra: (),
        }
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

impl<T> SizeCache<T> {
    /// Creates a new sized cache
    pub fn new_extra(value: usize, constrained: bool, extra: T) -> Self {
        Self {
            value,
            constrained,
            extra,
        }
    }

    /// Creates a new bi-dimensional cache.
    ///
    /// Similar to `build()`, but includes the extra field.
    pub fn build_extra(size: Vec2, req: Vec2, extra: XY<T>) -> XY<Self> {
        XY::zip3(size, req, extra)
            .map(|(size, req, extra)| SizeCache::new_extra(size, size >= req, extra))
    }

    /// Returns `true` if `self` is still valid for the given `request`.
    pub fn accept(self, request: usize) -> bool {
        match (request, self.value) {
            // Request a smaller size than last time? Hell no!
            (r, v) if r < v => false,
            // Request exactly what we had last time? Sure!
            (r, v) if r == v => true,
            // Request more than we had last time? Maybe?
            _ => !self.constrained,
        }
    }

    /// Returns the value in the cache.
    pub fn value(self) -> usize {
        self.value
    }
}
