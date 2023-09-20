#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeHint {
    pub min: i32,
    pub preferred: i32,
    pub is_fixed: bool,
}
