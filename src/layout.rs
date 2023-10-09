pub mod grid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeHints {
    // TODO: PhysicalPixels
    pub min: i32,
    pub preferred: i32,
    pub is_fixed: bool,
}

// impl SizeHint {
//     pub fn new_fallback() -> Self {
//         SizeHint {
//             min: 48,
//             preferred: 48,
//             is_fixed: true,
//         }
//     }
// }

pub const FALLBACK_SIZE_HINT: i32 = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeHintMode {
    Min,
    Preferred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct LayoutItemOptions {
    // alignment, priority, stretch, etc.
}
