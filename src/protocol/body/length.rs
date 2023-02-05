#[derive(Debug, Clone, Copy)]
pub enum BodyLength {
    Fix(usize),
    Chunked,
}

impl BodyLength {
    pub fn empty() -> Self {
        Self::Fix(0)
    }

    pub fn fix(length: usize) -> Self {
        Self::Fix(length)
    }

    pub fn chunked() -> Self {
        Self::Chunked
    }
}
