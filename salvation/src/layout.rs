use {
    self::grid::GridAxisOptions,
    crate::types::{PhysicalPixels, PpxSuffix},
    std::{
        cmp::{max, min},
        ops::RangeInclusive,
    },
};

pub mod grid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeHints {
    pub min: PhysicalPixels,
    pub preferred: PhysicalPixels,
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
pub const FALLBACK_SIZE_HINTS: SizeHints = SizeHints {
    min: PhysicalPixels::from_i32(FALLBACK_SIZE_HINT),
    preferred: PhysicalPixels::from_i32(FALLBACK_SIZE_HINT),
    is_fixed: true,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SizeHintMode {
    Min,
    Preferred,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LayoutItemOptions {
    pub x: LayoutItemAxisOptions,
    pub y: LayoutItemAxisOptions,
}

impl LayoutItemOptions {
    pub fn from_pos_in_grid(pos_x: i32, pos_y: i32) -> Self {
        Self {
            x: LayoutItemAxisOptions::new(pos_x),
            y: LayoutItemAxisOptions::new(pos_y),
        }
    }

    pub fn is_in_grid(&self) -> bool {
        self.x.pos_in_grid.is_some() && self.y.pos_in_grid.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LayoutItemAxisOptions {
    // row or column
    pub pos_in_grid: Option<RangeInclusive<i32>>,
    pub alignment: Option<Alignment>,
    pub is_fixed: Option<bool>,
    // TODO: alignment, priority, stretch, etc.
}

impl LayoutItemAxisOptions {
    pub fn new(pos: i32) -> Self {
        Self {
            pos_in_grid: Some(pos..=pos),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    Start,
    Middle,
    End,
}

pub(crate) fn fair_split(count: i32, total: PhysicalPixels) -> Vec<PhysicalPixels> {
    if count == 0 {
        return Vec::new();
    }
    let per_item = (total.to_i32() as f32) / (count as f32);
    let mut prev = 0.ppx();
    let mut results = Vec::new();
    for i in 1..=count {
        let next = PhysicalPixels::from_i32((per_item * (i as f32)).round() as i32);
        results.push(next - prev);
        prev = next;
    }
    results
}

#[derive(Debug)]
pub(crate) struct LayoutItem {
    pub(crate) size_hints: SizeHints,
    // TODO: params
}

#[derive(Debug)]
pub(crate) struct SolveLayoutOutput {
    pub(crate) sizes: Vec<PhysicalPixels>,
    pub(crate) padding: PhysicalPixels,
    pub(crate) spacing: PhysicalPixels,
}

// TODO: chose min/preferred spacing and padding
// TODO: support spanned items
pub(crate) fn solve_layout(
    items: &[LayoutItem],
    total: PhysicalPixels,
    options: &GridAxisOptions,
) -> SolveLayoutOutput {
    let mut output = SolveLayoutOutput {
        sizes: Vec::new(),
        padding: options.preferred_padding,
        spacing: options.preferred_spacing - options.border_collapse,
    };
    if items.is_empty() {
        return output;
    }
    let total_preferred = items
        .iter()
        .map(|item| item.size_hints.preferred)
        .sum::<PhysicalPixels>()
        + 2 * options.preferred_padding
        + items.len().saturating_sub(1) as i32
            * (options.preferred_spacing - options.border_collapse);
    if total_preferred == total {
        // Available size is exactly equal to the requested size.
        output.sizes = items.iter().map(|item| item.size_hints.preferred).collect();
        return output;
    } else if total_preferred > total {
        // Available size is less than the preferred size. Scaling down flexible items.
        let total_min = items
            .iter()
            .map(|item| item.size_hints.min)
            .sum::<PhysicalPixels>()
            + 2 * options.min_padding
            + items.len().saturating_sub(1) as i32
                * max(0.ppx(), options.min_spacing - options.border_collapse);
        let factor = if total_preferred == total_min {
            0.0
        } else {
            (total - total_min).to_i32() as f32 / (total_preferred - total_min).to_i32() as f32
        };
        output.padding = options.min_padding
            + PhysicalPixels::from_i32(
                // TODO: add PhysicalPixels::mul_f32_round method
                ((options.preferred_padding - options.min_padding).to_i32() as f32 * factor).round()
                    as i32,
            );
        output.spacing = options.min_spacing
            + PhysicalPixels::from_i32(
                ((options.preferred_spacing - options.min_spacing).to_i32() as f32 * factor).round()
                    as i32,
            );
        let mut remaining =
            total - output.padding * 2 - output.spacing * items.len().saturating_sub(1) as i32;
        for item in items {
            let item_size = item.size_hints.min
                + PhysicalPixels::from_i32(
                    ((item.size_hints.preferred - item.size_hints.min).to_i32() as f32 * factor)
                        .round() as i32,
                );
            let item_size = min(item_size, remaining);
            output.sizes.push(item_size);
            remaining -= item_size;
            if remaining == 0.ppx() {
                break;
            }
        }
    } else if total_preferred < total {
        let num_flexible = items
            .iter()
            .filter(|item| !item.size_hints.is_fixed)
            .count() as i32;
        let mut remaining =
            total - output.padding * 2 - output.spacing * items.len().saturating_sub(1) as i32;
        let mut extras = fair_split(num_flexible, max(0.ppx(), total - total_preferred));
        for item in items {
            let item_size = if item.size_hints.is_fixed {
                item.size_hints.preferred
            } else {
                item.size_hints.preferred + extras.pop().unwrap()
            };
            let item_size = min(item_size, remaining);
            output.sizes.push(item_size);
            remaining -= item_size;
            if remaining == 0.ppx() {
                break;
            }
        }
    }
    while output.sizes.len() < items.len() {
        output.sizes.push(0.ppx());
    }

    output
}
