//! Gradients
use crate::theme::Color;
use crate::{Vec2, XY};

/// RGB color.
///
/// If `T = u8` this is a 24-bit color.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Rgb<T = u8> {
    /// Red component.
    pub r: T,

    /// Green component.
    pub g: T,

    /// Blue component.
    pub b: T,
}

impl<T> From<[T; 3]> for Rgb<T> {
    fn from(o: [T; 3]) -> Self {
        let [r, g, b] = o;
        Self { r, g, b }
    }
}

impl From<u32> for Rgb<u8> {
    fn from(hex: u32) -> Self {
        Self::from_u32(hex)
    }
}

impl From<Rgb<u8>> for Rgb<f32> {
    fn from(o: Rgb<u8>) -> Self {
        o.as_f32()
    }
}

impl<T> Rgb<T> {
    /// Create a new Rgb from individual r, g and b values.
    pub fn new(r: T, g: T, b: T) -> Self {
        Self { r, g, b }
    }

    /// Run a closure on each component pair.
    pub fn zip_map<U, V>(a: Self, b: Rgb<U>, mut f: impl FnMut(T, U) -> V) -> Rgb<V> {
        Self::zip(a, b).map(|(a, b)| f(a, b))
    }

    /// Zip two Rgb into a single Rgb of tuples.
    pub fn zip<U>(a: Self, b: Rgb<U>) -> Rgb<(T, U)> {
        Rgb {
            r: (a.r, b.r),
            g: (a.g, b.g),
            b: (a.b, b.b),
        }
    }

    /// Apply a closure on each component.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Rgb<U> {
        Rgb {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
        }
    }
}

impl Rgb<f32> {
    /// Casts each component to u8.
    pub fn as_u8(self) -> Rgb<u8> {
        self.map(|x| x as u8)
    }

    /// Convert to a Color.
    pub fn as_color(self) -> Color {
        self.as_u8().as_color()
    }
}

impl Rgb<(f32, f32)> {
    /// Interpolate each component individually.
    pub fn interpolate(self, x: f32) -> Rgb<f32> {
        self.map(|(a, b)| a * (1f32 - x) + b * x)
    }
}

impl Rgb<(u8, u8)> {
    /// Cast each component to a tuple of f32.
    pub fn as_f32(self) -> Rgb<(f32, f32)> {
        self.map(|(x, y)| (x as f32, y as f32))
    }
}

impl Rgb<u8> {
    /// Convert to a Color.
    pub fn as_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }

    /// Return a Rgb using the lowest 24 bits.
    ///
    /// This can parse hex codes like `0xFF0000`.
    pub const fn from_u32(hex: u32) -> Self {
        let r = ((hex & 0xFF0000) >> 16) as u8;
        let g = ((hex & 0x00FF00) >> 8) as u8;
        let b = (hex & 0x0000FF) as u8;
        Self { r, g, b }
    }

    /// Cast each component to f32.
    pub fn as_f32(self) -> Rgb<f32> {
        self.map(|x| x as f32)
    }

    /// Returns a pure red RGB color.
    pub const fn red() -> Self {
        Self::from_u32(0xFF0000)
    }

    /// Returns a pure green RGB color.
    pub const fn green() -> Self {
        Self::from_u32(0x00FF00)
    }

    /// Returns a pure blue RGB color.
    pub const fn blue() -> Self {
        Self::from_u32(0x0000FF)
    }

    /// Returns a pure yellow RGB color.
    pub const fn yellow() -> Self {
        Self::from_u32(0xFFFF00)
    }
    /// Returns a pure magenta RGB color.
    pub const fn magenta() -> Self {
        Self::from_u32(0xFF00FF)
    }
    /// Returns a pure cyan RGB color.
    pub const fn cyan() -> Self {
        Self::from_u32(0x00FFFF)
    }
    /// Returns a pure white RGB color.
    pub const fn white() -> Self {
        Self::from_u32(0xFFFFFF)
    }
    /// Returns a pure black RGB color.
    pub const fn black() -> Self {
        Self::from_u32(0x000000)
    }
}

/// A linear gradient interpolating between 0 and 1.
pub struct Linear {
    /// Color for the start of the gradient.
    pub start: Rgb<f32>,

    // No allocation for simple start/end gradients.
    /// List of (position, color) intermediate points in the gradient.
    ///
    /// Positions should be in [0, 1].
    /// The values should be sorted by position.
    pub middle: Vec<(f32, Rgb<f32>)>,

    /// Color for the end of the gradient.
    pub end: Rgb<f32>,
}

impl Linear {
    /// Create a simple gradient with only a start and end colors.
    pub fn new(start: impl Into<Rgb<f32>>, end: impl Into<Rgb<f32>>) -> Self {
        let start = start.into();
        let end = end.into();
        Linear {
            start,
            end,
            middle: Vec::new(),
        }
    }

    /// Interpolate the color for the given position.
    pub fn interpolate(&self, x: f32) -> Rgb<f32> {
        // Find the segment
        if x <= 0f32 {
            return self.start;
        }
        if x >= 1f32 {
            return self.end;
        }

        let mut last = (0f32, self.start);
        for point in self.points() {
            if x > point.0 {
                last = point;
                continue;
            }

            let d = point.0 - last.0;
            let x = if d == 0f32 { 0f32 } else { (x - last.0) / d };

            return Rgb::zip(last.1, point.1).interpolate(x);
        }

        panic!("X has an invalid value (NaN?): {x:?}");
    }

    /// Iterates on the points of this gradient.
    pub fn points(&self) -> impl Iterator<Item = (f32, Rgb<f32>)> + '_ {
        std::iter::once((0f32, self.start))
            .chain(self.middle.iter().copied())
            .chain(std::iter::once((1f32, self.end)))
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
        let to_corner = self.center.map(|x| 0.5f32 + (x - 0.5f32).abs()) * size_f32;
        let max_distance = (to_corner.map(|x| x as isize).sq_norm() as f32).sqrt();

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
    /// 0 = vertical.
    pub angle_rad: f32,

    /// The gradient to apply following the gradient angle.
    pub gradient: Linear,
}

/// Something that can interpolate.
pub trait Interpolator {
    /// Get the color for the given position, given the total size.
    fn interpolate(&self, pos: Vec2, size: Vec2) -> Rgb<f32>;
}

impl Interpolator for Angled {
    fn interpolate(&self, pos: Vec2, size: Vec2) -> Rgb<f32> {
        // The starting corner depends on the angle.
        // angle_rad should be in [0, pi/2].
        let max_distance = Self::distance_along(self.angle_rad, size);

        // Compute the distance along the axis.
        // This means rotate and check the y value.
        let d = Self::distance_along(self.angle_rad, pos);

        self.gradient.interpolate(d / max_distance)
    }
}
impl Angled {
    fn distance_along(angle: f32, size: Vec2) -> f32 {
        if angle < 0f32 {
            return Self::distance_along(angle + std::f32::consts::FRAC_PI_2, size.swap());
        }

        if angle > std::f32::consts::PI {
            // Over 180 deg rotation
            return Self::distance_along(angle - std::f32::consts::PI, size);
        }

        if angle > std::f32::consts::FRAC_PI_2 {
            return Self::distance_along(angle - std::f32::consts::FRAC_PI_2, size.swap());
        }

        size.map(|x| x as f32).rotated(angle).y
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
        // TODO: handle size = 0 or 1 in any axis.
        // Size=0 => doesn't matter
        // Size=1 => ??? first value?
        let pos = pos.map(|x| x as f32) / size.map(|x| (x - 1) as f32);

        let top = Linear::new(self.top_left, self.top_right).interpolate(pos.x);
        let bottom = Linear::new(self.bottom_left, self.bottom_right).interpolate(pos.x);

        Linear::new(top, bottom).interpolate(pos.y)
    }
}
