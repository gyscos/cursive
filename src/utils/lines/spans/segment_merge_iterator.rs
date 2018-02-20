use super::Segment;

pub struct SegmentMergeIterator<I> {
    current: Option<Segment>,
    inner: I,
}

impl<I> SegmentMergeIterator<I> {
    pub fn new(inner: I) -> Self {
        SegmentMergeIterator {
            inner,
            current: None,
        }
    }
}

impl<I> Iterator for SegmentMergeIterator<I>
where
    I: Iterator<Item = Segment>,
{
    type Item = Segment;

    fn next(&mut self) -> Option<Self::Item> {
        // Always work on an segment.
        if self.current.is_none() {
            self.current = self.inner.next();
            self.current?;
        }

        // Keep growing our current segment until we find something else.
        loop {
            match self.inner.next() {
                None => return self.current.take(),
                Some(next) => {
                    if next.span_id == self.current.unwrap().span_id {
                        let current = self.current.as_mut().unwrap();
                        current.end = next.end;
                        current.width += next.width;
                    } else {
                        let current = self.current.take();
                        self.current = Some(next);
                        return current;
                    }
                }
            }
        }
    }
}
