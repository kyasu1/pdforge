use printpdf::*;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct OpBuffer {
    pub buffer: Vec<Vec<Op>>,
}

impl OpBuffer {
    pub fn insert(&mut self, page: usize, mut ops: Vec<Op>) {
        // if page < self.buffer.len() {
        //     self.buffer[page].append(&mut ops)
        // } else {
        //     self.buffer.push(ops);
        // }
        if page >= self.buffer.len() {
            self.buffer.resize(page + 1, Vec::new());
        }
        self.buffer[page].append(&mut ops);
    }
}
