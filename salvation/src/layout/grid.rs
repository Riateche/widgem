use {
    super::{Alignment, SizeHintMode, SizeHints},
    crate::{
        key::Key,
        layout::{fair_split, solve_layout},
        types::{Rect, Size},
        widgets::{Child, WidgetExt},
    },
    itertools::Itertools,
    log::warn,
    std::{
        cmp::{max, min},
        collections::{BTreeMap, HashMap},
        ops::RangeInclusive,
    },
};

#[derive(Debug, Clone)]
pub struct GridOptions {
    pub x: GridAxisOptions,
    pub y: GridAxisOptions,
}

impl GridOptions {
    pub const ZERO: Self = GridOptions {
        x: GridAxisOptions {
            min_padding: 0,
            min_spacing: 0,
            preferred_padding: 0,
            preferred_spacing: 0,
            border_collapse: 0,
            alignment: Alignment::Start,
        },
        y: GridAxisOptions {
            min_padding: 0,
            min_spacing: 0,
            preferred_padding: 0,
            preferred_spacing: 0,
            border_collapse: 0,
            alignment: Alignment::Start,
        },
    };
}

#[derive(Debug, Clone)]
pub struct GridAxisOptions {
    pub min_padding: i32,
    pub min_spacing: i32,
    pub preferred_padding: i32,
    pub preferred_spacing: i32,
    pub border_collapse: i32,
    pub alignment: Alignment,
}

fn size_hint(
    items: &[(RangeInclusive<i32>, i32)],
    options: &GridAxisOptions,
    mode: SizeHintMode,
) -> i32 {
    let mut max_per_column = BTreeMap::new();
    let mut spanned = Vec::new();
    for (pos, hint) in items {
        if pos.start() == pos.end() {
            let value = max_per_column.entry(*pos.start()).or_default();
            *value = max(*value, *hint);
        } else if pos.start() > pos.end() {
            warn!("invalid pos_in_grid range");
            continue;
        } else {
            spanned.push((pos, *hint));
        }
    }
    for (range, hint) in spanned {
        let current: i32 = range
            .clone()
            .map(|pos| max_per_column.get(&pos).copied().unwrap_or(0))
            .sum();
        if hint > current {
            let extra_per_column = fair_split(*range.end() - *range.start() + 1, hint - current);
            for (pos, extra) in range.clone().zip(extra_per_column) {
                *max_per_column.entry(pos).or_default() += extra;
            }
        }
    }
    let (padding, spacing) = match mode {
        SizeHintMode::Min => (options.min_padding, options.min_spacing),
        SizeHintMode::Preferred => (options.preferred_padding, options.preferred_spacing),
    };
    max_per_column.values().sum::<i32>()
        + 2 * padding
        + max_per_column.len().saturating_sub(1) as i32 * (spacing - options.border_collapse)
}

pub fn size_hint_x(items: &mut BTreeMap<Key, Child>, options: &GridOptions) -> SizeHints {
    let mut min_items = Vec::new();
    let mut preferred_items = Vec::new();
    let mut all_fixed = true;
    for item in items.values_mut() {
        if !(item.widget.common().layout_item_options.is_in_grid()
            && !item.widget.common().is_window_root
            && item.widget.common().is_self_visible)
        {
            continue;
        }

        let hints = item.widget.size_hint_x();
        let pos_in_grid = item
            .widget
            .common()
            .layout_item_options
            .x
            .pos_in_grid
            .clone()
            .unwrap();
        min_items.push((pos_in_grid.clone(), hints.min));
        preferred_items.push((pos_in_grid, hints.preferred));

        let is_fixed = item
            .widget
            .common()
            .layout_item_options
            .x
            .is_fixed
            .unwrap_or(hints.is_fixed);

        if !is_fixed {
            all_fixed = false;
        }
    }
    SizeHints {
        min: size_hint(&min_items, &options.x, SizeHintMode::Min),
        preferred: size_hint(&preferred_items, &options.x, SizeHintMode::Preferred),
        is_fixed: all_fixed,
    }
}

pub fn size_hint_y(
    items: &mut BTreeMap<Key, Child>,
    options: &GridOptions,
    size_x: i32,
    mode: SizeHintMode,
) -> i32 {
    let x_layout = x_layout(items, &options.x, size_x);
    let mut item_data = Vec::new();
    for (key, item) in items.iter_mut() {
        if !item.widget.common().layout_item_options.is_in_grid()
            || item.widget.common().is_window_root
            || !item.widget.common().is_self_visible
        {
            continue;
        }
        let Some(item_size_x) = x_layout.child_sizes.get(key) else {
            continue;
        };
        let pos = item
            .widget
            .common()
            .layout_item_options
            .y
            .pos_in_grid
            .clone()
            .unwrap();
        let hints = item.widget.size_hint_y(*item_size_x, mode);
        item_data.push((pos, hints));
    }
    size_hint(&item_data, &options.y, mode)
}

pub fn size_y_fixed(items: &mut BTreeMap<Key, Child>, _options: &GridOptions) -> bool {
    items.values_mut().all(|item| {
        !item.widget.common().layout_item_options.is_in_grid()
            || item.widget.common().is_window_root
            || !item.widget.common().is_self_visible
            || item
                .widget
                .common()
                .layout_item_options
                .y
                .is_fixed
                .unwrap_or_else(|| item.widget.size_y_fixed())
    })
}

struct XLayout {
    padding: i32,
    spacing: i32,
    column_sizes: BTreeMap<i32, i32>,
    child_sizes: HashMap<Key, i32>,
}

fn x_layout(items: &mut BTreeMap<Key, Child>, options: &GridAxisOptions, size_x: i32) -> XLayout {
    let mut hints_per_column = BTreeMap::new();
    for item in items.values_mut() {
        if !item.widget.common().layout_item_options.is_in_grid()
            || item.widget.common().is_window_root
            || !item.widget.common().is_self_visible
        {
            continue;
        }
        let Some(pos) = item
            .widget
            .common()
            .layout_item_options
            .x
            .pos_in_grid
            .clone()
        else {
            continue;
        };
        if pos.start() != pos.end() {
            warn!("spanned items are not supported yet");
        }
        let pos = *pos.start();
        let mut hints = item.widget.size_hint_x();
        if let Some(is_fixed) = item.widget.common().layout_item_options.x.is_fixed {
            hints.is_fixed = is_fixed;
        }
        let column_hints = hints_per_column.entry(pos).or_insert(hints);
        column_hints.min = max(column_hints.min, hints.min);
        column_hints.preferred = max(column_hints.preferred, hints.preferred);
        column_hints.is_fixed = column_hints.is_fixed && hints.is_fixed;
    }
    let layout_items = hints_per_column
        .values()
        .map(|hints| super::LayoutItem { size_hints: *hints })
        .collect_vec();
    let output = solve_layout(&layout_items, size_x, options);
    let column_sizes: BTreeMap<_, _> = hints_per_column.keys().copied().zip(output.sizes).collect();
    let mut child_sizes = HashMap::new();
    for (key, item) in items.iter_mut() {
        if !item.widget.common().layout_item_options.is_in_grid()
            || item.widget.common().is_window_root
            || !item.widget.common().is_self_visible
        {
            continue;
        }
        let Some(pos) = item
            .widget
            .common()
            .layout_item_options
            .x
            .pos_in_grid
            .clone()
        else {
            continue;
        };
        if pos.start() != pos.end() {
            warn!("spanned items are not supported yet");
        }
        let Some(column_size) = column_sizes.get(pos.start()) else {
            warn!("missing column data for existing child");
            continue;
        };
        let child_size = if item
            .widget
            .common()
            .layout_item_options
            .x
            .is_fixed
            .unwrap_or_else(|| item.widget.size_hint_x().is_fixed)
        {
            let hint = item.widget.size_hint_x().preferred;
            min(hint, *column_size)
        } else {
            *column_size
        };
        child_sizes.insert(key.clone(), child_size);
    }
    XLayout {
        padding: output.padding,
        spacing: output.spacing,
        column_sizes,
        child_sizes,
    }
}

pub fn layout(
    items: &mut BTreeMap<Key, Child>,
    options: &GridOptions,
    size: Size,
) -> BTreeMap<Key, Rect> {
    let x_layout = x_layout(items, &options.x, size.x);
    let mut hints_per_row = BTreeMap::new();
    for (key, item) in items.iter_mut() {
        if !item.widget.common().layout_item_options.is_in_grid()
            || item.widget.common().is_window_root
        {
            //|| !item.widget.common().is_self_visible {
            continue;
        }
        let Some(pos) = item
            .widget
            .common()
            .layout_item_options
            .y
            .pos_in_grid
            .clone()
        else {
            continue;
        };
        if pos.start() != pos.end() {
            warn!("spanned items are not supported yet");
        }
        let Some(item_size_x) = x_layout.child_sizes.get(key) else {
            continue;
        };
        let pos = *pos.start();
        let mut hints = item.widget.size_hints_y(*item_size_x);
        if let Some(is_fixed) = item.widget.common().layout_item_options.y.is_fixed {
            hints.is_fixed = is_fixed;
        }
        let row_hints = hints_per_row.entry(pos).or_insert(hints);
        row_hints.min = max(row_hints.min, hints.min);
        row_hints.preferred = max(row_hints.preferred, hints.preferred);
        row_hints.is_fixed = row_hints.is_fixed && hints.is_fixed;
        // TODO: deduplicate
    }
    let layout_items = hints_per_row
        .values()
        .map(|hints| super::LayoutItem { size_hints: *hints })
        .collect_vec();
    let output_y = solve_layout(&layout_items, size.y, &options.y);
    let row_sizes: BTreeMap<_, _> = hints_per_row.keys().copied().zip(output_y.sizes).collect();
    let positions_x = positions(
        &x_layout.column_sizes,
        x_layout.padding,
        x_layout.spacing,
        size.x,
        options.x.alignment,
    );
    let positions_y = positions(
        &row_sizes,
        output_y.padding,
        output_y.spacing,
        size.y,
        options.y.alignment,
    );
    let mut result = BTreeMap::new();
    for (key, item) in items.iter_mut() {
        // if !item.widget.common().is_self_visible {
        //     continue;
        // }
        let Some(pos_x) = item
            .widget
            .common()
            .layout_item_options
            .x
            .pos_in_grid
            .clone()
        else {
            continue;
        };
        let Some(pos_y) = item
            .widget
            .common()
            .layout_item_options
            .y
            .pos_in_grid
            .clone()
        else {
            continue;
        };
        let Some(cell_pos_x) = positions_x.get(pos_x.start()) else {
            continue;
        };
        let Some(cell_pos_y) = positions_y.get(pos_y.start()) else {
            warn!("missing item in positions_y");
            continue;
        };
        let Some(size_x) = x_layout.child_sizes.get(key) else {
            warn!("missing item in x_layout.child_sizes");
            continue;
        };
        let Some(row_size) = row_sizes.get(pos_y.start()) else {
            warn!("missing item in row_sizes");
            continue;
        };
        let size_y = if item
            .widget
            .common()
            .layout_item_options
            .y
            .is_fixed
            .unwrap_or_else(|| item.widget.size_y_fixed())
        {
            min(
                *row_size,
                item.widget.size_hint_y(*size_x, SizeHintMode::Preferred),
            )
        } else {
            *row_size
        };
        result.insert(
            key.clone(),
            Rect::from_xywh(*cell_pos_x, *cell_pos_y, *size_x, size_y),
        );
    }
    result
}

fn positions(
    sizes: &BTreeMap<i32, i32>,
    padding: i32,
    spacing: i32,
    total_available: i32,
    alignment: Alignment,
) -> BTreeMap<i32, i32> {
    let mut pos = padding;
    let total_taken: i32 = sizes.values().copied().sum();
    let available_for_items =
        total_available - 2 * padding - spacing * sizes.len().saturating_sub(1) as i32;
    match alignment {
        Alignment::Start => {}
        Alignment::Middle => {
            pos += (available_for_items - total_taken) / 2;
        }
        Alignment::End => {
            pos += available_for_items - total_taken;
        }
    }
    let mut result = BTreeMap::new();
    for (num, size) in sizes {
        result.insert(*num, pos);
        pos += *size + spacing;
    }
    result
}
