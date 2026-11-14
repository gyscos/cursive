use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Default)]
struct Inner {
    values: VecDeque<f64>,
    capacity: Option<usize>,
}

impl Inner {
    fn trim(&mut self) {
        if let Some(capacity) = self.capacity {
            let extra = self.values.len().saturating_sub(capacity);
            self.values.drain(..extra);
        }
    }
}

/// A shared sequence of numbers, to be drawn by data views.
///
/// This is the data behind [`Chart`](crate::views::Chart), [`Sparkline`](crate::views::Sparkline)
/// and [`Heatmap`](crate::views::Heatmap). Values are ordered from the oldest to the most recent.
///
/// Cloning a `Series` gives another handle to the same data: give one to a view, keep the other
/// to add values later, possibly from another thread.
///
/// Note that adding values does not by itself refresh the screen: see
/// [`Cursive::set_fps`](crate::Cursive::set_fps) or [`Cursive::cb_sink`](crate::Cursive::cb_sink).
///
/// # Examples
///
/// ```rust
/// use cursive_core::views::Series;
///
/// // Only keep the most recent 100 values.
/// let series = Series::with_capacity(100);
/// // Give a handle to a view...
/// let for_the_view = series.clone();
///
/// // Later, possibly in a different thread
/// series.push(12.5);
/// assert_eq!(series.last(), Some(12.5));
/// ```
#[derive(Clone, Debug, Default)]
pub struct Series {
    inner: Arc<Mutex<Inner>>,
}

impl Series {
    /// Creates a new empty series, which keeps every value.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new empty series, which only keeps the most recent `capacity` values.
    pub fn with_capacity(capacity: usize) -> Self {
        let series = Self::new();
        series.inner.lock().capacity = Some(capacity);
        series
    }

    /// Returns the maximum number of values this series keeps, if there is a limit.
    pub fn capacity(&self) -> Option<usize> {
        self.inner.lock().capacity
    }

    /// Adds a value at the end of the series.
    ///
    /// If the series is full, the oldest value is removed.
    pub fn push(&self, value: f64) {
        let mut inner = self.inner.lock();
        inner.values.push_back(value);
        inner.trim();
    }

    /// Adds several values at the end of the series.
    pub fn extend(&self, values: impl IntoIterator<Item = f64>) {
        let mut inner = self.inner.lock();
        inner.values.extend(values);
        inner.trim();
    }

    /// Replaces all the values in the series.
    pub fn set(&self, values: impl IntoIterator<Item = f64>) {
        let mut inner = self.inner.lock();
        inner.values.clear();
        inner.values.extend(values);
        inner.trim();
    }

    /// Removes all the values.
    pub fn clear(&self) {
        self.inner.lock().values.clear();
    }

    /// Returns the number of values in the series.
    pub fn len(&self) -> usize {
        self.inner.lock().values.len()
    }

    /// Returns `true` if the series has no value.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().values.is_empty()
    }

    /// Returns the most recent value.
    pub fn last(&self) -> Option<f64> {
        self.inner.lock().values.back().copied()
    }

    /// Returns the value `n` steps before the most recent one.
    ///
    /// `ago(0)` is the same as [`last`](Self::last).
    pub fn ago(&self, n: usize) -> Option<f64> {
        let inner = self.inner.lock();
        let index = inner.values.len().checked_sub(n.checked_add(1)?)?;
        inner.values.get(index).copied()
    }

    /// Returns the most recent `n` values (or fewer if the series is shorter), oldest first.
    pub fn tail(&self, n: usize) -> Vec<f64> {
        let inner = self.inner.lock();
        let skip = inner.values.len().saturating_sub(n);
        inner.values.iter().skip(skip).copied().collect()
    }

    /// Returns all the values, oldest first.
    pub fn to_vec(&self) -> Vec<f64> {
        self.inner.lock().values.iter().copied().collect()
    }
}

impl From<Vec<f64>> for Series {
    fn from(values: Vec<f64>) -> Self {
        let series = Series::new();
        series.inner.lock().values = values.into();
        series
    }
}

impl From<&[f64]> for Series {
    fn from(values: &[f64]) -> Self {
        values.to_vec().into()
    }
}

impl<const N: usize> From<[f64; N]> for Series {
    fn from(values: [f64; N]) -> Self {
        values.to_vec().into()
    }
}

impl From<&Series> for Series {
    fn from(series: &Series) -> Self {
        series.clone()
    }
}

impl FromIterator<f64> for Series {
    fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_share_the_data() {
        let series = Series::new();
        let other = series.clone();
        assert!(other.is_empty());
        assert_eq!(other.last(), None);

        series.push(1.0);
        series.extend([2.0, 3.0]);
        assert_eq!(other.to_vec(), [1.0, 2.0, 3.0]);
        assert_eq!(other.len(), 3);
        assert_eq!(other.last(), Some(3.0));

        other.clear();
        assert!(series.is_empty());
    }

    #[test]
    fn capacity_drops_the_oldest_values() {
        let series = Series::with_capacity(3);
        assert_eq!(series.capacity(), Some(3));
        assert_eq!(Series::new().capacity(), None);

        series.extend([1.0, 2.0]);
        series.push(3.0);
        series.push(4.0);
        assert_eq!(series.to_vec(), [2.0, 3.0, 4.0]);

        series.extend([5.0, 6.0, 7.0, 8.0]);
        assert_eq!(series.to_vec(), [6.0, 7.0, 8.0]);

        series.set([1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(series.to_vec(), [3.0, 4.0, 5.0]);

        let nothing = Series::with_capacity(0);
        nothing.push(1.0);
        assert!(nothing.is_empty());
    }

    #[test]
    fn tail_and_ago() {
        let series = Series::from([1.0, 2.0, 3.0, 4.0]);
        assert_eq!(series.tail(2), [3.0, 4.0]);
        assert_eq!(series.tail(10), [1.0, 2.0, 3.0, 4.0]);
        assert!(series.tail(0).is_empty());

        assert_eq!(series.ago(0), Some(4.0));
        assert_eq!(series.ago(3), Some(1.0));
        assert_eq!(series.ago(4), None);
    }

    #[test]
    fn conversions() {
        assert_eq!(Series::from(vec![1.0, 2.0]).to_vec(), [1.0, 2.0]);
        assert_eq!(Series::from(&[1.0, 2.0][..]).to_vec(), [1.0, 2.0]);
        let series: Series = (0..3).map(f64::from).collect();
        assert_eq!(series.to_vec(), [0.0, 1.0, 2.0]);
        assert_eq!(Series::from(&series).to_vec(), series.to_vec());
    }

    #[test]
    fn ago_with_huge_offsets_is_none() {
        let series = Series::from([1.0, 2.0]);
        assert_eq!(series.ago(usize::MAX), None);
        assert_eq!(series.ago(usize::MAX - 1), None);
        assert_eq!(series.ago(2), None);
        assert_eq!(series.ago(1), Some(1.0));
    }

    #[test]
    fn zero_capacity_keeps_nothing() {
        let series = Series::with_capacity(0);
        series.extend([1.0, 2.0, 3.0]);
        series.set([1.0]);
        series.push(4.0);
        assert!(series.is_empty());
        assert_eq!(series.tail(5), Vec::<f64>::new());
        assert_eq!(series.ago(0), None);
    }
}
