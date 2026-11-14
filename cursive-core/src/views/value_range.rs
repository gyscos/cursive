use std::ops::RangeInclusive;

/// The values at both ends of an axis.
///
/// Created by [`ValueRange::resolve`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    /// Value at the start of the axis.
    pub min: f64,
    /// Value at the end of the axis. Always larger than `min`.
    pub max: f64,
}

impl Bounds {
    /// Returns where `value` is between the bounds: 0 for `min`, 1 for `max`.
    ///
    /// The result is clamped between 0 and 1. Invalid values (`NaN`) give 0.
    pub fn ratio(&self, value: f64) -> f32 {
        let ratio = (value - self.min) / (self.max - self.min);
        if ratio.is_nan() {
            0.0
        } else {
            ratio.clamp(0.0, 1.0) as f32
        }
    }
}

/// How data views pick the range of values they show.
///
/// Each end of the range is either fixed, or automatic: computed from the data.
///
/// # Examples
///
/// ```rust
/// use cursive_core::views::ValueRange;
///
/// // A fixed range. Values outside of it are clamped.
/// let range = ValueRange::from(0.0..=100.0);
/// assert_eq!(range.resolve([20.0, 250.0]).max, 100.0);
///
/// // An automatic range, exactly fitting the data.
/// let bounds = ValueRange::auto().resolve([20.0, 250.0]);
/// assert_eq!((bounds.min, bounds.max), (20.0, 250.0));
///
/// // Starts from zero, ends at the next multiple of 100 above the data.
/// let range = ValueRange::auto().include(0.0).step(100.0);
/// let bounds = range.resolve([20.0, 250.0]);
/// assert_eq!((bounds.min, bounds.max), (0.0, 300.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValueRange {
    min: Option<f64>,
    max: Option<f64>,
    // Smallest range the automatic bounds must cover.
    include: Option<(f64, f64)>,
    // Values to treat as part of the data.
    cover: Option<(f64, f64)>,
    step: Option<f64>,
    margin: f64,
}

impl Default for ValueRange {
    /// An automatic range that includes zero.
    fn default() -> Self {
        Self::auto().include(0.0)
    }
}

impl ValueRange {
    /// A range entirely computed from the data: from its lowest to its highest value.
    pub const fn auto() -> Self {
        ValueRange {
            min: None,
            max: None,
            include: None,
            cover: None,
            step: None,
            margin: 0.0,
        }
    }

    /// A fixed range.
    pub const fn fixed(min: f64, max: f64) -> Self {
        Self::auto().min(min).max(max)
    }

    /// Fixes the start of the range.
    #[must_use]
    pub const fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    /// Fixes the end of the range.
    #[must_use]
    pub const fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Makes sure automatic bounds include the given value, whatever the data.
    ///
    /// For example, `include(0.0)` gives a range that starts from zero when all the values are
    /// positive. This can be called several times.
    ///
    /// The value is only a lower or upper limit for the bounds: no [`margin`](Self::margin) is
    /// added next to it. To behave as if the data really went up to a value, including its
    /// margin, use [`cover`](Self::cover).
    ///
    /// ```rust
    /// use cursive_core::views::ValueRange;
    ///
    /// let range = ValueRange::auto().include(0.0).include(2.0).margin(0.1);
    /// // The data is below 2: the range stops at 2, without margin.
    /// let bounds = range.resolve([0.5]);
    /// assert_eq!((bounds.min, bounds.max), (0.0, 2.0));
    /// ```
    #[must_use]
    pub const fn include(mut self, value: f64) -> Self {
        self.include = Some(match self.include {
            None => (value, value),
            Some((low, high)) => (low.min(value), high.max(value)),
        });
        self
    }

    /// Behaves as if `value` was part of the data: the range contains it, and margins apply to it.
    ///
    /// This is different from [`include`](Self::include), which does not add a margin next to
    /// the value. Use `cover` for "show the data, with room for at least up to this value".
    ///
    /// This can be called several times.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use cursive_core::views::ValueRange;
    ///
    /// // Starts from zero, and goes at least to 2 (plus a margin).
    /// let range = ValueRange::auto().include(0.0).cover(2.0).margin(0.1);
    ///
    /// let bounds = range.resolve([0.5]);
    /// assert_eq!((bounds.min, bounds.max), (0.0, 2.2));
    ///
    /// // Larger data goes beyond.
    /// let bounds = range.resolve([4.0]);
    /// assert_eq!((bounds.min, bounds.max), (0.0, 4.4));
    /// ```
    #[must_use]
    pub const fn cover(mut self, value: f64) -> Self {
        self.cover = Some(match self.cover {
            None => (value, value),
            Some((low, high)) => (low.min(value), high.max(value)),
        });
        self
    }

    /// Rounds automatic bounds outward, to multiples of `step`.
    ///
    /// This keeps the range stable when the data changes a little, and gives round values at
    /// both ends. A bound that already is a multiple of the step (give or take the rounding
    /// error of floating-point numbers, so `0.3` is one for a step of `0.1`) stays where it is.
    ///
    /// A step that is not finite and positive is ignored.
    #[must_use]
    pub const fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }

    /// Leaves some room between the data and automatic bounds.
    ///
    /// `margin` is a fraction of the range covered by the data: with `0.1`, the highest value
    /// is drawn at about 90% of the available space.
    ///
    /// There is no margin next to a bound that only comes from [`include`](Self::include).
    #[must_use]
    pub const fn margin(mut self, margin: f64) -> Self {
        self.margin = margin;
        self
    }

    /// Computes the actual bounds for the given data.
    ///
    /// Invalid and infinite values are ignored. The result always has finite bounds with
    /// `max > min`: when there is nothing to show a range of (no data, or a single value), the
    /// range goes one unit (or one [step](Self::step)) above the minimum. For values too large
    /// for that to make a difference, the room grows with their magnitude, and at the very top
    /// of the finite numbers it is made below the maximum instead.
    ///
    /// The exception is a fixed bound given as `NaN` or infinite: it is used as it is, and the
    /// result may then not satisfy any of that.
    pub fn resolve(&self, values: impl IntoIterator<Item = f64>) -> Bounds {
        let data = values.into_iter().filter(|value| value.is_finite()).fold(
            None,
            |range: Option<(f64, f64)>, value| match range {
                None => Some((value, value)),
                Some((low, high)) => Some((low.min(value), high.max(value))),
            },
        );

        let data = match (data, self.cover) {
            (Some(data), Some(cover)) => Some((data.0.min(cover.0), data.1.max(cover.1))),
            (data, cover) => data.or(cover),
        };

        // What the automatic bounds need to cover.
        let (low, high) = match (data, self.include) {
            (Some(data), Some(include)) => (data.0.min(include.0), data.1.max(include.1)),
            (Some(range), None) | (None, Some(range)) => range,
            (None, None) => (0.0, 0.0),
        };

        // Only add a margin where the data reaches the bound.
        // (`margin * (high - low)` would be NaN for a zero margin and an infinite span.)
        let span = high - low;
        let margin = if self.margin > 0.0 && span.is_finite() {
            self.margin * span
        } else {
            0.0
        };
        let touches = |bound: f64, pick: fn((f64, f64)) -> f64| data.map(pick) == Some(bound);
        let mut min = if touches(low, |data| data.0) {
            low - margin
        } else {
            low
        };
        let mut max = if touches(high, |data| data.1) {
            high + margin
        } else {
            high
        };
        // A margin can push a bound out of the finite numbers: keep the one without margin.
        if !min.is_finite() {
            min = low;
        }
        if !max.is_finite() {
            max = high;
        }

        let step = self.step.filter(|step| step.is_finite() && *step > 0.0);
        if let Some(step) = step {
            // The rounding is not allowed to leave the finite numbers either.
            let rounded = round_down(min, step);
            min = if rounded.is_finite() { rounded } else { min };
            let rounded = round_up(max, step);
            max = if rounded.is_finite() { rounded } else { max };
        }

        let mut min = self.min.unwrap_or(min);
        let mut max = self.max.unwrap_or(max);
        if max.partial_cmp(&min) != Some(std::cmp::Ordering::Greater) {
            // Nothing to show a range of: make some room above, one unit or one step.
            // For large values that is lost in rounding, so the room grows with the magnitude.
            let room = step.unwrap_or(1.0).max(min.abs() * f64::EPSILON);
            let above = min + room;
            if above.is_finite() {
                max = above;
            } else if min.is_finite() {
                // At the top of the finite numbers: the room is below.
                max = min;
                min -= room;
            } else {
                // A fixed bound that is NaN or infinite: nothing sensible to do.
                max = above;
            }
        }

        Bounds { min, max }
    }
}

/// Returns `true` if `value` is a multiple of `step`, up to a few ulps.
///
/// Dividing by a step like `0.1` is not exact: `0.3 / 0.1` is `2.9999999999999996`, and a
/// floor would put the bound one step too low.
fn is_multiple_of(value: f64, step: f64) -> bool {
    let ratio = value / step;
    // A few ulps, relative to the size of the ratio. Non-finite ratios give `false`.
    (ratio.round() - ratio).abs() <= 4.0 * f64::EPSILON * ratio.abs()
}

/// Rounds down to a multiple of `step`, unless `value` already is one.
fn round_down(value: f64, step: f64) -> f64 {
    let ratio = value / step;
    if is_multiple_of(value, step) || !ratio.is_finite() {
        value
    } else {
        ratio.floor() * step
    }
}

/// Rounds up to a multiple of `step`, unless `value` already is one.
fn round_up(value: f64, step: f64) -> f64 {
    let ratio = value / step;
    if is_multiple_of(value, step) || !ratio.is_finite() {
        value
    } else {
        ratio.ceil() * step
    }
}

impl From<RangeInclusive<f64>> for ValueRange {
    fn from(range: RangeInclusive<f64>) -> Self {
        Self::fixed(*range.start(), *range.end())
    }
}

impl From<(f64, f64)> for ValueRange {
    fn from((min, max): (f64, f64)) -> Self {
        Self::fixed(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(range: ValueRange, values: &[f64]) -> (f64, f64) {
        let bounds = range.resolve(values.iter().copied());
        (bounds.min, bounds.max)
    }

    #[test]
    fn fixed() {
        assert_eq!(bounds((0.0..=100.0).into(), &[-5.0, 500.0]), (0.0, 100.0));
        assert_eq!(bounds((1.0, 2.0).into(), &[]), (1.0, 2.0));
        // Only one end fixed.
        assert_eq!(bounds(ValueRange::auto().min(0.0), &[5.0, 9.0]), (0.0, 9.0));
        assert_eq!(
            bounds(ValueRange::auto().max(10.0), &[5.0, 9.0]),
            (5.0, 10.0)
        );
    }

    #[test]
    fn auto() {
        assert_eq!(bounds(ValueRange::auto(), &[3.0, -1.0, 2.0]), (-1.0, 3.0));
        // Invalid values are ignored.
        let values = [1.0, f64::NAN, f64::INFINITY, 2.0];
        assert_eq!(bounds(ValueRange::auto(), &values), (1.0, 2.0));
        // The default includes zero.
        assert_eq!(bounds(ValueRange::default(), &[3.0, 5.0]), (0.0, 5.0));
        assert_eq!(bounds(ValueRange::default(), &[-3.0, -5.0]), (-5.0, 0.0));
    }

    #[test]
    fn include_step_margin() {
        let range = ValueRange::auto().include(0.0).include(2.0);
        assert_eq!(bounds(range, &[0.5, 1.0]), (0.0, 2.0));
        assert_eq!(bounds(range, &[0.5, 4.0]), (0.0, 4.0));
        assert_eq!(bounds(range, &[]), (0.0, 2.0));

        let range = ValueRange::auto().include(0.0).step(50.0);
        assert_eq!(bounds(range, &[20.0, 101.0]), (0.0, 150.0));
        assert_eq!(bounds(range, &[-1.0, 100.0]), (-50.0, 100.0));

        // The margin only applies on the side of the data.
        let range = ValueRange::auto().include(0.0).margin(0.1);
        assert_eq!(bounds(range, &[50.0, 100.0]), (0.0, 110.0));
        let range = ValueRange::auto().margin(0.5);
        assert_eq!(bounds(range, &[10.0, 20.0]), (5.0, 25.0));

        // Margin first, then rounding.
        let range = ValueRange::auto().include(0.0).margin(0.1).step(1000.0);
        assert_eq!(bounds(range, &[950.0]), (0.0, 2000.0));
    }

    #[test]
    fn cover_applies_margins() {
        let range = ValueRange::auto().include(0.0).cover(2.0).margin(0.1);
        let (min, max) = bounds(range, &[0.5]);
        assert_eq!(min, 0.0);
        assert!((max - 2.2).abs() < 1e-9, "{max}");
        let (min, max) = bounds(range, &[4.0]);
        assert_eq!(min, 0.0);
        assert!((max - 4.4).abs() < 1e-9, "{max}");

        // The same with `include` has no margin next to the included value.
        let range = ValueRange::auto().include(0.0).include(2.0).margin(0.1);
        assert_eq!(bounds(range, &[0.5]), (0.0, 2.0));
    }

    #[test]
    fn cover_without_data() {
        let range = ValueRange::auto().include(0.0).cover(2.0).margin(0.1);
        let (min, max) = bounds(range, &[]);
        assert_eq!(min, 0.0);
        assert!((max - 2.2).abs() < 1e-9, "{max}");

        // Alone, it is like a single value.
        assert_eq!(bounds(ValueRange::auto().cover(5.0), &[]), (5.0, 6.0));
        // Invalid data is ignored.
        let range = ValueRange::auto().include(0.0).cover(3.0);
        assert_eq!(bounds(range, &[f64::NAN]), (0.0, 3.0));
    }

    #[test]
    fn cover_several_times() {
        let range = ValueRange::auto().cover(-1.0).cover(3.0).cover(1.0);
        assert_eq!(bounds(range, &[]), (-1.0, 3.0));
        assert_eq!(bounds(range, &[0.0]), (-1.0, 3.0));
        assert_eq!(bounds(range, &[10.0]), (-1.0, 10.0));
        let range = range.margin(0.25);
        assert_eq!(bounds(range, &[]), (-2.0, 4.0));
    }

    #[test]
    fn cover_with_step_and_fixed_bounds() {
        let range = ValueRange::auto().include(0.0).cover(2.0).step(5.0);
        assert_eq!(bounds(range, &[0.5]), (0.0, 5.0));
        assert_eq!(bounds(range, &[7.0]), (0.0, 10.0));

        // Margin first, then rounding.
        let range = ValueRange::auto()
            .include(0.0)
            .cover(95.0)
            .margin(0.1)
            .step(50.0);
        assert_eq!(bounds(range, &[1.0]), (0.0, 150.0));

        // Fixed bounds win.
        let range = ValueRange::auto().include(0.0).cover(2.0).max(1.0);
        assert_eq!(bounds(range, &[0.5]), (0.0, 1.0));
        let range = ValueRange::auto().cover(2.0).min(-3.0);
        assert_eq!(bounds(range, &[0.5]), (-3.0, 2.0));
        let range = ValueRange::fixed(0.0, 10.0).cover(50.0);
        assert_eq!(bounds(range, &[1.0]), (0.0, 10.0));
    }

    #[test]
    fn never_empty() {
        assert_eq!(bounds(ValueRange::auto(), &[]), (0.0, 1.0));
        assert_eq!(bounds(ValueRange::auto(), &[7.0]), (7.0, 8.0));
        assert_eq!(
            bounds(ValueRange::auto().step(50.0), &[100.0]),
            (100.0, 150.0)
        );
        assert_eq!(bounds(ValueRange::fixed(5.0, 5.0), &[]), (5.0, 6.0));
        assert_eq!(bounds(ValueRange::fixed(5.0, 1.0), &[]), (5.0, 6.0));
    }

    #[test]
    fn ratio() {
        let bounds = Bounds {
            min: 10.0,
            max: 20.0,
        };
        assert_eq!(bounds.ratio(10.0), 0.0);
        assert_eq!(bounds.ratio(15.0), 0.5);
        assert_eq!(bounds.ratio(20.0), 1.0);
        assert_eq!(bounds.ratio(-100.0), 0.0);
        assert_eq!(bounds.ratio(100.0), 1.0);
        assert_eq!(bounds.ratio(f64::NAN), 0.0);
    }

    #[test]
    fn step_keeps_values_that_are_multiples_of_a_fractional_step() {
        // 0.3 / 0.1 is 2.9999999999999996: it is still a multiple of 0.1.
        let b = ValueRange::auto().step(0.1).resolve([0.3, 0.7]);
        assert_eq!((b.min, b.max), (0.3, 0.7));
        // 0.07 / 0.01 is 7.000000000000001.
        let b = ValueRange::auto().step(0.01).resolve([0.0, 0.07]);
        assert_eq!((b.min, b.max), (0.0, 0.07));
        for (low, high, step) in [
            (0.1, 0.9, 0.1),
            (0.2, 0.6, 0.2),
            (1.1, 3.3, 1.1),
            (0.15, 0.45, 0.05),
            (-0.7, -0.3, 0.1),
            (-0.3, 0.7, 0.1),
            (1e6 * 0.3, 1e6 * 0.7, 1e5),
            (3e-7, 7e-7, 1e-7),
        ] {
            let b = ValueRange::auto().step(step).resolve([low, high]);
            assert_eq!((b.min, b.max), (low, high), "step {step}");
        }
    }

    #[test]
    fn step_still_rounds_outward() {
        let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-12, "{a} != {b}");
        let b = ValueRange::auto().step(0.1).resolve([0.31, 0.69]);
        close(b.min, 0.3);
        close(b.max, 0.7);
        let b = ValueRange::auto().step(0.1).resolve([0.29, 0.71]);
        close(b.min, 0.2);
        close(b.max, 0.8);
        let b = ValueRange::auto().step(0.01).resolve([-0.0001, 0.0701]);
        close(b.min, -0.01);
        close(b.max, 0.08);
        // Integers, with a step that is not exact in binary either.
        let b = ValueRange::auto().step(5.0).resolve([-0.5, 12.0]);
        assert_eq!((b.min, b.max), (-5.0, 15.0));
        let b = ValueRange::auto().step(1.0).resolve([3.5, 4.0]);
        assert_eq!((b.min, b.max), (3.0, 4.0));
        // Huge values, and steps so small that the ratio overflows.
        let b = ValueRange::auto().step(1.0).resolve([1e300, 2e300]);
        assert_eq!((b.min, b.max), (1e300, 2e300));
        let b = ValueRange::auto().step(1e-300).resolve([1e300, 2e300]);
        assert_eq!((b.min, b.max), (1e300, 2e300));
    }

    #[test]
    fn constant_data_always_has_room() {
        // Ordinary values: exactly one more, or one step more.
        let b = ValueRange::auto().resolve([5.0, 5.0]);
        assert_eq!((b.min, b.max), (5.0, 6.0));
        let b = ValueRange::auto().step(10.0).resolve([5.0]);
        assert_eq!((b.min, b.max), (0.0, 10.0));
        let b = ValueRange::auto().step(0.25).resolve([1.0]);
        assert_eq!((b.min, b.max), (1.0, 1.25));

        // Where adding 1 changes nothing, scale with the magnitude.
        let large = [
            1e16,
            9.1e15,
            1e17,
            1.7e18,
            1e300,
            f64::MAX / 2.0,
            f64::MAX,
            -1e17,
            -f64::MAX,
            f64::MIN_POSITIVE,
        ];
        for value in large {
            for range in [
                ValueRange::auto(),
                ValueRange::auto().step(1.0),
                ValueRange::auto().step(1e-3),
                ValueRange::auto().min(value),
                ValueRange::auto().max(value),
                ValueRange::auto().min(value).max(value),
                ValueRange::auto().include(value).margin(0.5),
                ValueRange::fixed(value, value),
            ] {
                let b = range.resolve([value, value]);
                assert!(b.max > b.min, "{value}: {range:?} gives {b:?}");
                assert!(b.min.is_finite() && b.max.is_finite(), "{value}: {b:?}");
                // The data is still inside, and shown at a valid position.
                assert!((0.0..=1.0).contains(&b.ratio(value)));
                assert!(b.min <= value && value <= b.max, "{value}: {b:?}");
            }
        }
        // Next to the largest value, the room is made on the other side.
        let b = ValueRange::fixed(f64::MAX, f64::MAX).resolve([]);
        assert_eq!(b.max, f64::MAX);
        assert!(b.min < b.max);
    }

    #[test]
    fn extreme_data_gives_finite_bounds() {
        for range in [
            ValueRange::auto(),
            ValueRange::auto().margin(0.1),
            ValueRange::auto().include(0.0).step(100.0).margin(0.1),
            ValueRange::auto().step(f64::MAX),
        ] {
            let b = range.resolve([f64::MIN, f64::MAX]);
            assert!(b.min.is_finite() && b.max.is_finite(), "{range:?}: {b:?}");
            assert!(b.max > b.min, "{range:?}: {b:?}");
            assert!(b.min <= f64::MIN || b.min.is_finite());
        }
    }

    #[test]
    fn every_combination_of_settings_gives_a_range() {
        let datas: [&[f64]; 8] = [
            &[],
            &[0.0],
            &[5.0],
            &[-5.0, 5.0],
            &[f64::NAN],
            &[1e300, -1e300],
            &[0.3, 0.7],
            &[1.0, f64::INFINITY],
        ];
        let mins = [None, Some(-3.0), Some(0.0), Some(10.0)];
        let maxs = [None, Some(-3.0), Some(0.0), Some(10.0)];
        let includes = [None, Some(0.0), Some(7.0)];
        let covers = [None, Some(-2.0), Some(20.0)];
        let steps = [
            None,
            Some(0.0),
            Some(-5.0),
            Some(0.1),
            Some(50.0),
            Some(f64::NAN),
        ];
        let margins = [0.0, 0.1, -1.0, 2.0];
        for data in datas {
            for min in mins {
                for max in maxs {
                    for include in includes {
                        for cover in covers {
                            for step in steps {
                                for margin in margins {
                                    let mut r = ValueRange::auto().margin(margin);
                                    if let Some(v) = min {
                                        r = r.min(v);
                                    }
                                    if let Some(v) = max {
                                        r = r.max(v);
                                    }
                                    if let Some(v) = include {
                                        r = r.include(v);
                                    }
                                    if let Some(v) = cover {
                                        r = r.cover(v);
                                    }
                                    if let Some(v) = step {
                                        r = r.step(v);
                                    }
                                    let b = r.resolve(data.iter().copied());
                                    let ctx = format!("{r:?} {data:?} -> {b:?}");
                                    assert!(b.max > b.min, "{ctx}");
                                    if let Some(v) = min {
                                        assert_eq!(b.min, v, "{ctx}");
                                    }
                                    if let Some(v) = max
                                        && v > b.min
                                    {
                                        assert_eq!(b.max, v, "{ctx}");
                                    }
                                    assert!((0.0..=1.0).contains(&b.ratio(1.0)), "{ctx}");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
