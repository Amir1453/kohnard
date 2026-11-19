use crate::span::Span;

/// The parsing context that is passed on to the parser.
pub(crate) struct ParseCtxt {
    offset: u32,
    // next_expr: Cell<usize>,
}

impl ParseCtxt {
    /// Constructs a new ParseCtxt, with an offset.
    pub const fn new(offset: u32) -> Self {
        Self { offset }
    }

    /// Converts a file-local byte offset into a global SourceMap byte offset by adding the file’s base offset.
    pub fn position(&self, local: usize) -> u32 {
        let local = u32::try_from(local).expect("source position exceeds u32 range");
        self.offset
            .checked_add(local)
            .expect("source position exceeds u32 address space")
    }

    /// Constructs a Span.
    pub fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.position(start), self.position(end))
    }
}
