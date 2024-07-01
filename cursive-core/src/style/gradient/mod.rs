//! Gradients
//!
//! This module defines a few types to help work with gradients:
//!
//! * [`Linear`] is a 1-dimensional (linear) piecewise gradient: given a float value between 0 and
//!   1, it returns the interpolated [`crate::style::Rgb`] color at that point.
//! * A few 2D gradients assign float values between 0 and 1 to each cell of a grid, and internally
//!   use a `Linear` gradient to convert that to a color.
//!   * [`Angled`] applies its linear gradient on the 2D grid at an angle.
//!   * [`Radial`] applies its linear gradient according to the distance from a center.
//!   * [`Bilinear`] uses bilinear interpolation between the 4 corners to compute the color for
//!     each cell.
//!
//! Note that this module works with `Rgb<f32>`, where each color has a f32 value between 0 and 1.
//! Various conversions to/from `Rgb<u8>` and [`crate::style::Color`] are available.
use crate::{style::Rgb, Vec2, XY};

/// A 2D color distribution.
///
/// Types implementing this trait can assign a color to each point of a grid.
pub trait Interpolator {
    /// Get the color for the given position, given the total size of the area to cover.
    ///
    /// The resulting value uses floats between 0 and 1.
    fn interpolate(&self, pos: Vec2, size: Vec2) -> Rgb<f32>;
}

/// Dynamic interpolator.
///
/// Convenient alias to make sure the proper bounds are applied.
pub type Dynterpolator = Box<dyn Interpolator + Send + Sync>;

impl Interpolator for Dynterpolator {
    fn interpolate(&self, pos: Vec2, size: Vec2) -> Rgb<f32> {
        // Deref first into the ref, then into the box.
        (**self).interpolate(pos, size)
    }
}

/// A linear gradient interpolating color for floats between 0 and 1.
pub struct Linear {
    /// List of (position, color) intermediate points in the gradient.
    ///
    /// Invariant: the values should be sorted by position.
    points: Vec<(f32, Rgb<f32>)>,
}

fn sort_points(points: &mut [(f32, Rgb<f32>)]) {
    points.sort_by(|(time_a, _), (time_b, _)| time_a.partial_cmp(time_b).unwrap());
}

impl Linear {
    /// Create a gradient from the given points.
    pub fn new(mut points: Vec<(f32, Rgb<f32>)>) -> Self {
        sort_points(&mut points);
        Self { points }
    }

    /// Creates a simple gradient between the two given colors.
    pub fn simple<S, E>(start: S, end: E) -> Self
    where
        S: Into<Rgb<f32>>,
        E: Into<Rgb<f32>>,
    {
        let start = start.into();
        let end = end.into();
        Self::evenly_spaced(&[start, end])
    }

    /// Returns a linear gradient mirrored from `self`.
    pub fn mirror(mut self) -> Self {
        self.rescale(|t| 1.0 - t);

        self
    }

    /// Rescale this gradient to cover `[0: 1]`.
    pub fn normalize(&mut self) {
        if self.points.is_empty() {
            return;
        }

        if self.points.len() == 1 {
            self.points[0].0 = 0f32;
            return;
        }

        let start = self.points[0].0;
        let end = self.points.last().unwrap().0;

        if start == end {
            // If all the points have the same time, re-scale them evenly over [0:1].
            let step = (self.points.len() as f32 - 1f32).recip();
            for (i, &mut (ref mut time, _)) in self.points.iter_mut().enumerate() {
                *time = step * i as f32;
            }
        } else {
            self.rescale(|x| (x - start) / (end - start));
        }
    }

    /// Adjusts the position of the intermediate points.
    pub fn rescale<F>(&mut self, mut f: F)
    where
        F: FnMut(f32) -> f32,
    {
        for &mut (ref mut time, _) in &mut self.points {
            *time = f(*time);
        }

        sort_points(&mut self.points);
    }

    /// Create a simple gradient with evenly spaced colors.
    ///
    /// * Returns a flat black gradient if `colors` is empty.
    /// * Returns a constant "gradient" (same start and end) if `colors.len() == 1`.
    /// * Returns a piecewise gradient between all colors otherwise.
    pub fn evenly_spaced<R: Copy + Into<Rgb<f32>>>(colors: &[R]) -> Self {
        let step = 1f32 / (colors.len() - 1) as f32;
        let colors = colors.iter().copied().map(Into::into).enumerate();
        let points = colors.map(|(i, color)| (step * i as f32, color)).collect();
        Self { points }
    }

    /// Returns a simple black-to-white gradient.
    pub fn black_to_white() -> Self {
        Self::simple(Rgb::black(), Rgb::white())
    }

    /// Returns a rainbow gradient.
    pub fn rainbow() -> Self {
        // These values are derived from the color spectrum
        let mut res = Self::new(vec![
            (4.0, Rgb::violet().into()),
            (4.7, Rgb::blue().into()),
            (4.9, Rgb::cyan().into()),
            (5.3, Rgb::green().into()),
            (5.75, Rgb::yellow().into()),
            (6.1, Rgb::orange().into()),
            (6.9, Rgb::red().into()),
        ]);
        res.normalize();
        res.mirror()
    }

    // TODO: Implement conversion from an iterator of (f32, Rgb), using an offset + rescaling

    // TODO: Add some preset gradients (rainbow, fire, ...)
    // For example from uigradients.com

    /// Interpolate the color for the given position.
    ///
    /// The resulting value uses floats between 0 and 1.
    pub fn interpolate(&self, x: f32) -> Rgb<f32> {
        if self.points.is_empty() {
            return Rgb::black().as_f32();
        }
        if self.points.len() == 1 {
            return self.points[0].1;
        }

        if x <= self.points[0].0 {
            return self.points[0].1;
        }

        let last = self.points.last().unwrap();
        if x >= last.0 {
            return last.1;
        }

        let mut last = self.points[0];
        for point in self.points() {
            if x > point.0 {
                last = *point;
                continue;
            }

            // x is between the previous step and this one.

            let d = point.0 - last.0;
            let x = if d == 0f32 { 0f32 } else { (x - last.0) / d };

            return Rgb::zip(last.1, point.1).interpolate(x);
        }

        panic!("X has an invalid value (NaN?): {x:?}");
    }

    /// Iterates on the points of this gradient.
    pub fn points(&self) -> &[(f32, Rgb<f32>)] {
        &self.points
    }
}

impl From<(Rgb<f32>, Rgb<f32>)> for Linear {
    fn from((start, end): (Rgb<f32>, Rgb<f32>)) -> Self {
        Self::evenly_spaced(&[start, end])
    }
}

impl From<[Rgb<f32>; 2]> for Linear {
    fn from([start, end]: [Rgb<f32>; 2]) -> Self {
        Self::evenly_spaced(&[start, end])
    }
}

impl From<(Rgb<u8>, Rgb<u8>)> for Linear {
    fn from((start, end): (Rgb<u8>, Rgb<u8>)) -> Self {
        Self::evenly_spaced(&[start.as_f32(), end.as_f32()])
    }
}

impl From<[Rgb<u8>; 2]> for Linear {
    fn from([start, end]: [Rgb<u8>; 2]) -> Self {
        Self::evenly_spaced(&[start.as_f32(), end.as_f32()])
    }
}

/// Radial gradient.
pub struct Radial {
    /// Where the gradient starts.
    ///
    /// This should be in [0, 1] for each component, as a ratio of the total size.
    pub center: XY<f32>,

    /// The gradient to apply according to the distance from the center.
    pub gradient: Linear,
}

impl Interpolator for Radial {
    fn interpolate(&self, pos: Vec2, size: Vec2) -> Rgb<f32> {
        let size_f32 = size.map(|x| x as f32);

        // Find the further corner from `size`.
        //
        // TODO: cache this for the same value of `size`?
        // (Define a type that combines the gradient and the size, to be re-used for a draw cycle?)
        let to_corner = self.center.map(|x| 0.5f32 + (x - 0.5f32).abs()) * size_f32;
        let max_distance = (to_corner.map(|x| x as isize).sq_norm() as f32)
            .sqrt()
            .max(1.0);

        let center = (self.center * size_f32).map(|x| x as isize);

        let sq_dist = (center - pos.signed()).sq_norm();
        let dist = (sq_dist as f32).sqrt();

        self.gradient.interpolate(dist / max_distance)
    }
}

/// An angled linear gradient.
pub struct Angled {
    /// Angle of the gradient in radians.
    ///
    /// * 0 = vertical from top to bottom.
    /// * Pi/2 = horizontal from left to right.
    /// * Pi = vertical from bottom to top.
    /// * 3.Pi/2 = vertical from bottom to top.
    pub angle_rad: f32,

    /// The gradient to apply following the gradient angle.
    pub gradient: Linear,
}

impl Interpolator for Angled {
    fn interpolate(&self, mut pos: Vec2, mut size: Vec2) -> Rgb<f32> {
        use std::f32::consts::{FRAC_PI_2, PI, TAU};

        let mut angle = self.angle_rad;

        // First, normalize the angle: add/remove TAU until we are in [0, TAU[
        while angle < 0f32 {
            angle += TAU;
        }

        while angle >= TAU {
            angle -= TAU;
        }

        // Now there are 4 quadrants we need to handle: [0:PI/2[, [PI/2:PI[, [PI:3PI/2[, [3PI/2, TAU[
        // TODO: Refactor a bit to only need `pos` at the end (build a 3x3 matrix to apply?)
        match angle {
            _ if angle < FRAC_PI_2 => (),
            _ if angle < PI => {
                // Here, pos.x = max.x - pos.
                pos = Vec2::new(size.y - pos.y, pos.x);
                size = size.swap();
                angle -= FRAC_PI_2;
            }
            _ if angle < PI + FRAC_PI_2 => {
                pos = size - pos;
                angle -= PI;
            }
            _ => {
                pos = Vec2::new(pos.y, size.x - pos.x);
                size = size.swap();
                angle -= PI + FRAC_PI_2;
            }
        }

        let d = pos.map(|x| x as f32).rotated(angle).y;

        // Define max distance as always at least 1.0 to prevent divide-by-0
        let max = size.map(|x| x as f32).rotated(angle).y.max(1.0);

        self.gradient.interpolate(d / max)
    }
}

/// Bilinear gradient.
///
/// This applies bilinear interpolation to a rectangle with a given color at each corner.
pub struct Bilinear {
    /// Color for the top-left corner.
    pub top_left: Rgb<f32>,
    /// Color for the bottom-left corner.
    pub bottom_left: Rgb<f32>,
    /// Color for the top-right corner.
    pub top_right: Rgb<f32>,
    /// Color for the bottom-right corner.
    pub bottom_right: Rgb<f32>,
}

impl Interpolator for Bilinear {
    fn interpolate(&self, pos: Vec2, size: Vec2) -> Rgb<f32> {
        if !Vec2::new(2, 2).fits_in(size) {
            // Size=0 => doesn't matter
            // Size=1 => ??? first value?
            return self.top_left;
        }

        // Here size >= (2.2), so (size - (1,1)) > 0
        let pos = pos.map(|x| x as f32) / size.map(|x| (x - 1) as f32);

        let top = Rgb::zip(self.top_left, self.top_right).interpolate(pos.x);
        let bottom = Rgb::zip(self.bottom_left, self.bottom_right).interpolate(pos.x);

        Rgb::zip(top, bottom).interpolate(pos.y)
    }
}
