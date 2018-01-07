use std::borrow::Cow;
use theme::Style;
use unicode_width::UnicodeWidthStr;
use xi_unicode::LineBreakLeafIter;

pub struct Span<'a> {
    text: Cow<'a, str>,
    width: usize,
    style: Style,
}

pub struct Row<'a> {
    spans: Vec<Span<'a>>,
    width: usize,
}

pub struct SpanLinesIterator<'a: 'b, 'b> {
    /// Input that we want to split
    content: &'b [Span<'a>],

    /// Available width
    width: usize,

    current_span: usize,
    offset: usize,
}

impl<'a: 'b, 'b> SpanLinesIterator<'a, 'b> {
    pub fn new(content: &'b [Span<'a>], width: usize) -> Self {
        SpanLinesIterator {
            content,
            width,
            current_span: 0,
            offset: 0,
        }
    }
}

// Intermediate representation of a Span, easier to manipulate.
struct Segment {
    span_id: usize,
    start: usize,
    end: usize,
    width: usize,
}

impl<'a, 'b> Iterator for SpanLinesIterator<'a, 'b> {
    type Item = Row<'a>;

    fn next(&mut self) -> Option<Row<'a>> {
        if self.current_span >= self.content.len() {
            return None;
        }

        let current_span = &self.content[self.current_span];

        let mut available = self.width;
        let mut iter = LineBreakLeafIter::new(&current_span.text, self.offset);

        let mut spans = Vec::new();
        let mut width = 0;

        // We'll build a list of segments.
        // There will be a 1-for-1 mapping from segments to spans.
        // But segments are easier to manipulate and extend for now.
        let mut segments: Vec<Segment> = Vec::new();

        // When a span does not end on a possible break, its last segment
        // can only be included depending on what comes after.
        // So we keep a list of consecutive segments ids without breaks.
        let mut carry_over: Vec<usize> = Vec::new();
        // Whenever a segment is accepted, all of these can be inserted too.

        'outer: for (span_id, span) in
            self.content.iter().enumerate().skip(self.current_span)
        {
            // Make a new segment!
            loop {
                // Get the next possible break point.
                let (pos, hard) = iter.next(&span.text);

                // Lookup the corresponding text segment.
                let segment = &span.text[self.offset..pos];
                let width = segment.width();

                // If it doesn't fit, it's time to go home.
                if width > available {
                    // Early return!
                    break 'outer;
                }

                available -= width;

                // It fits, but... for real?
                if pos == span.text.len() {
                    // It was too good to be true!
                    // It's just the end of a span, not an actual break.
                    // So save this stub for now, and move on to the next span.
                    carry_over.push(span_id);
                    // Start on the next span.
                    self.offset = 0;
                    break;
                }

                // We got it! We got a chunk!
                // First, append any carry-over segment
                for carry in carry_over.drain(..) {
                    // We need to include this entire segment.
                    if segments.last().map(|s| s.span_id) == Some(carry) {

                    } else {
                        segments.push(Segment {});
                    }
                }

                // Include the present segment.
                if pos != 0 {
                    segments.push(Segment {
                        span_id,
                        width,
                        start: self.offset,
                        end: pos,
                    });

                    self.offset = pos;
                }

                if hard {
                    // Stop here.
                    break 'outer;
                }
            }
        }

        loop {
            let current_span = &self.content[self.current_span];
            let (pos, hard) = iter.next(&current_span.text);

            // This is what we consider adding
            let text = &current_span.text[self.offset..pos];

            if hard {
                // Stop there!
                break;
            }
        }

        Some(Row { spans, width })
    }
}
