#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeHint {
    // TODO: PhysicalPixels
    pub min: i32,
    pub preferred: i32,
    pub is_fixed: bool,
}

impl SizeHint {
    pub fn new_fallback() -> Self {
        SizeHint {
            min: 48,
            preferred: 48,
            is_fixed: true,
        }
    }
}
