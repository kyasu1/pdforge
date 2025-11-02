use printpdf::*;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct OpBuffer {
    pub buffer: Vec<Vec<Op>>,
}

impl OpBuffer {
    /// Create a new OpBuffer with pre-allocated capacity for expected pages
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, page: usize, mut ops: Vec<Op>) {
        if page >= self.buffer.len() {
            self.buffer.resize(page + 1, Vec::new());
        }
        self.buffer[page].append(&mut ops);
    }

    /// Clear the buffer and free memory (useful for streaming)
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.buffer.shrink_to_fit();
    }

    /// Get the number of pages in the buffer
    pub fn page_count(&self) -> usize {
        self.buffer.len()
    }
}
