use anyhow::Result;
use salvation_macros::impl_with;
use std::cmp::{max, min};

use crate::{
    event::LayoutEvent,
    layout::SizeHint,
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
    let hint = child.widget.cached_size_hint_x();
    if hint.is_fixed {
        min(hint.preferred, layout_size_x)
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

    pub fn add_child(&mut self, widget: Box<dyn Widget>) {
        self.common.add_child(self.common.children.len(), widget);
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
            let child_hint_y = child.widget.cached_size_hint_y(child_size_x);
            sizes_x.push(child_size_x);
            items_y.push(LayoutItem {
                size_hint: child_hint_y,
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

    fn size_hint_x(&mut self) -> Result<SizeHint> {
        let mut r = SizeHint {
            min: 0,
            preferred: 0,
            is_fixed: true,
        };
        for child in &mut self.common.children {
            let child_hint = child.widget.cached_size_hint_x();
            r.min = max(r.min, child_hint.min);
            r.preferred = max(r.preferred, child_hint.preferred);
            if !child_hint.is_fixed {
                r.is_fixed = false;
            }
        }
        Ok(r)
    }
    fn size_hint_y(&mut self, size_x: i32) -> Result<SizeHint> {
        let mut r = SizeHint {
            min: 0,
            preferred: 0,
            is_fixed: true,
        };
        for (i, child) in self.common.children.iter_mut().enumerate() {
            let child_size_x = child_size_x(size_x, child);
            let child_hint = child.widget.cached_size_hint_y(child_size_x);
            if i != 0 {
                r.min += SPACING;
                r.preferred += SPACING;
            }
            r.min += child_hint.min;
            r.preferred += child_hint.preferred;
            if !child_hint.is_fixed {
                r.is_fixed = false;
            }
        }
        Ok(r)
    }
}

struct LayoutItem {
    size_hint: SizeHint,
    // TODO: params
}

#[allow(clippy::comparison_chain)]
fn solve_layout(items: &[LayoutItem], total: i32) -> Vec<i32> {
    if items.is_empty() {
        return Vec::new();
    }
    let total_preferred: i32 = items.iter().map(|item| item.size_hint.preferred).sum();
    let mut result = Vec::new();
    if total_preferred == total {
        return items.iter().map(|item| item.size_hint.preferred).collect();
    } else if total_preferred > total {
        let total_min: i32 = items.iter().map(|item| item.size_hint.min).sum();
        let factor = if total_preferred == total_min {
            0.0
        } else {
            (total - total_min) as f32 / (total_preferred - total_min) as f32
        };
        let mut remaining = total;
        for item in items {
            let item_size = item.size_hint.min
                + ((item.size_hint.preferred - item.size_hint.min) as f32 * factor).round() as i32;
            let item_size = min(item_size, remaining);
            result.push(item_size);
            remaining -= item_size;
            if remaining == 0 {
                break;
            }
        }
    } else if total_preferred < total {
        let num_flexible = items.iter().filter(|item| !item.size_hint.is_fixed).count() as i32;
        let mut remaining = total;
        let mut extras = fare_split(num_flexible, max(0, total - total_preferred));
        for item in items {
            let item_size = if item.size_hint.is_fixed {
                item.size_hint.preferred
            } else {
                item.size_hint.preferred + extras.pop().unwrap()
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
