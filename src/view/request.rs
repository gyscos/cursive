use vec::ToVec2;

/// Describe constraints on a view layout in one dimension.
#[derive(PartialEq,Clone,Copy)]
pub enum DimensionRequest {
    /// The view must use exactly the attached size.
    Fixed(usize),
    /// The view is free to choose its size if it stays under the limit.
    AtMost(usize),
    /// No clear restriction apply.
    Unknown,
}

impl DimensionRequest {
    /// Returns a new request, reduced from the original by the given offset.
    pub fn reduced(self, offset: usize) -> Self {
        match self {
            DimensionRequest::Fixed(w) => DimensionRequest::Fixed(w - offset),
            DimensionRequest::AtMost(w) => {
                DimensionRequest::AtMost(w - offset)
            }
            DimensionRequest::Unknown => DimensionRequest::Unknown,
        }
    }
}

/// Describes constraints on a view layout.
#[derive(PartialEq,Clone,Copy)]
pub struct SizeRequest {
    /// Restriction on the view width
    pub w: DimensionRequest,
    /// Restriction on the view height
    pub h: DimensionRequest,
}

impl SizeRequest {
    /// Returns a new SizeRequest, reduced from the original by the given offset.
    pub fn reduced<T: ToVec2>(self, offset: T) -> Self {
        let ov = offset.to_vec2();
        SizeRequest {
            w: self.w.reduced(ov.x),
            h: self.h.reduced(ov.y),
        }
    }

    /// Creates a new dummy request, with no restriction.
    pub fn dummy() -> Self {
        SizeRequest {
            w: DimensionRequest::Unknown,
            h: DimensionRequest::Unknown,
        }
    }
}
