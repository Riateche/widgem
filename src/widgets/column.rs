use anyhow::Result;
use salvation_macros::impl_with;
use std::cmp::{max, min};

use crate::{
    event::LayoutEvent,
    layout::{LayoutItemOptions, SizeHintMode, SizeHints},
    types::{Point, Rect, Size},
};

use super::{Widget, WidgetCommon, WidgetExt};

// TODO: get from style, apply scale
const SPACING: i32 = 10;

pub struct Column {
    // TODO: add layout options
    common: WidgetCommon,
}

fn child_size_x(layout_size_x: i32, child: &mut super::Child) -> i32 {
    if child.widget.cached_size_hint_x_fixed() {
        let hint = child.widget.cached_size_hint_x(SizeHintMode::Preferred);
        min(hint, layout_size_x)
    } else {
        layout_size_x
    }
}

#[impl_with]
impl Column {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            common: WidgetCommon::new(),
        }
    }

    pub fn add_child(&mut self, widget: Box<dyn Widget>, options: LayoutItemOptions) {
        self.common.add_child(widget, options);
        self.common.update();
    }
}

impl Widget for Column {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let rect_in_window = self.common().rect_in_window_or_err()?;
        let mut items_y = Vec::new();
        let mut sizes_x = Vec::new();
        for child in self.common.children.iter_mut() {
            let child_size_x = child_size_x(rect_in_window.size.x, child);
            let child_hints_y = child.widget.cached_size_hints_y(child_size_x);
            sizes_x.push(child_size_x);
            items_y.push(LayoutItem {
                size_hints: child_hints_y,
            });
        }
        let mut current_y = 0;
        // TODO: this is incorrect for extremely small sizes where not all items are visible
        let available_size_y =
            rect_in_window.size.y - self.common.children.len().saturating_sub(1) as i32 * SPACING;
        let solved = solve_layout(&items_y, available_size_y);
        for (i, (result_item, size_x)) in solved.into_iter().zip(sizes_x).enumerate() {
            if result_item == 0 {
                self.common.set_child_rect(i, None)?;
                continue;
            }
            if i != 0 {
                current_y += SPACING;
            }
            let child_rect = Rect {
                top_left: Point { x: 0, y: current_y },
                size: Size {
                    x: size_x,
                    y: result_item,
                },
            };
            self.common.set_child_rect(i, Some(child_rect))?;
            current_y = child_rect.bottom_right().y;
        }
        Ok(())
    }

    fn size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let mut r = 0;
        for child in &mut self.common.children {
            r = max(r, child.widget.cached_size_hint_x(mode));
        }
        Ok(r)
    }
    fn is_size_hint_x_fixed(&mut self) -> bool {
        self.common
            .children
            .iter_mut()
            .all(|child| child.widget.cached_size_hint_x_fixed())
    }
    fn is_size_hint_y_fixed(&mut self) -> bool {
        self.common
            .children
            .iter_mut()
            .all(|child| child.widget.cached_size_hint_y_fixed())
    }
    fn size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        let mut r = 0;
        for (i, child) in self.common.children.iter_mut().enumerate() {
            let child_size_x = child_size_x(size_x, child);
            if i != 0 {
                r += SPACING;
            }
            r += child.widget.cached_size_hint_y(child_size_x, mode);
        }
        Ok(r)
    }
}

struct LayoutItem {
    size_hints: SizeHints,
    // TODO: params
}

#[allow(clippy::comparison_chain)]
fn solve_layout(items: &[LayoutItem], total: i32) -> Vec<i32> {
    if items.is_empty() {
        return Vec::new();
    }
    let total_preferred: i32 = items.iter().map(|item| item.size_hints.preferred).sum();
    let mut result = Vec::new();
    if total_preferred == total {
        return items.iter().map(|item| item.size_hints.preferred).collect();
    } else if total_preferred > total {
        let total_min: i32 = items.iter().map(|item| item.size_hints.min).sum();
        let factor = if total_preferred == total_min {
            0.0
        } else {
            (total - total_min) as f32 / (total_preferred - total_min) as f32
        };
        let mut remaining = total;
        for item in items {
            let item_size = item.size_hints.min
                + ((item.size_hints.preferred - item.size_hints.min) as f32 * factor).round()
                    as i32;
            let item_size = min(item_size, remaining);
            result.push(item_size);
            remaining -= item_size;
            if remaining == 0 {
                break;
            }
        }
    } else if total_preferred < total {
        let num_flexible = items
            .iter()
            .filter(|item| !item.size_hints.is_fixed)
            .count() as i32;
        let mut remaining = total;
        let mut extras = fare_split(num_flexible, max(0, total - total_preferred));
        for item in items {
            let item_size = if item.size_hints.is_fixed {
                item.size_hints.preferred
            } else {
                item.size_hints.preferred + extras.pop().unwrap()
            };
            let item_size = min(item_size, remaining);
            result.push(item_size);
            remaining -= item_size;
            if remaining == 0 {
                break;
            }
        }
    }
    while result.len() < items.len() {
        result.push(0);
    }

    result
}

fn fare_split(count: i32, total: i32) -> Vec<i32> {
    if count == 0 {
        return Vec::new();
    }
    let per_item = (total as f32) / (count as f32);
    let mut prev = 0;
    let mut results = Vec::new();
    for i in 1..=count {
        let next = (per_item * (i as f32)).round() as i32;
        results.push(next - prev);
        prev = next;
    }
    results
}
