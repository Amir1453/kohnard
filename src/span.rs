//! Span information to know from which source and which line the changes came from.

use std::path::PathBuf;

use typed_index_collections::TiVec as IndexVec;

/// Span information for the AST.
/// At compile, every file is stitched together, with each of them having ranges.
/// The start and end are offset based on the file they came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    /// The byte offset to start from.
    start: u32,
    /// The byte offset that ends.
    end: u32,
}

impl Span {
    #[must_use]
    #[inline(always)]
    pub const fn new(start: u32, end: u32) -> Self {
        assert!(start <= end, "span start must not exceed its end");
        Self { start, end }
    }

    #[inline]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

crate::index::index_type!(FileId);

/// The SourceMap stores the source files that were processed during compilation.
/// We store the files
pub struct SourceMap {
    files: IndexVec<FileId, SourceFile>,
}

impl SourceMap {
    pub fn new(compilation_units: Vec<PathBuf>) -> anyhow::Result<Self> {
        let mut files = IndexVec::<FileId, SourceFile>::with_capacity(compilation_units.len());
        let mut file_offset = 0u32;

        for cu in compilation_units {
            let source = std::fs::read_to_string(&cu)?;
            let src_len = u32::try_from(source.len())?;

            let mut line_starts = vec![0];
            for (offset, byte) in source.bytes().enumerate() {
                if byte == b'\n' {
                    line_starts.push((offset + 1) as u32);
                }
            }

            let file = SourceFile {
                path: cu,
                source,
                line_starts,
                offset: file_offset,
            };

            files.push(file);

            file_offset = file_offset
                .checked_add(src_len)
                .and_then(|end| end.checked_add(1))
                .ok_or_else(|| anyhow::anyhow!("source map exceeds u32 address space"))?;
        }

        Ok(Self { files })
    }

    pub fn files(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter()
    }
}

/// Represents a source file.
pub struct SourceFile {
    /// Path to the source file.
    pub path: PathBuf,
    /// The source file stored as a String.
    pub source: String,
    /// Contains a vector of the byte offset of where each newline starts.
    pub line_starts: Vec<u32>,
    /// Contains the offset at which the file is stored.
    pub offset: u32,
}
