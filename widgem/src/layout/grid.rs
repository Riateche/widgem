use {
    super::{Alignment, SizeHintMode, SizeHints},
    crate::{
        layout::{fair_split, solve_layout},
        types::{PhysicalPixels, PpxSuffix, Rect},
        widgets::{Widget, WidgetAddress, WidgetExt, WidgetGeometry},
        RawWidgetId,
    },
    itertools::Itertools,
    log::warn,
    std::{
        cmp::{max, min},
        collections::{hash_map, BTreeMap, HashMap},
        ops::RangeInclusive,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GridOptions {
    pub x: GridAxisOptions,
    pub y: GridAxisOptions,
}

impl GridOptions {
    pub const ZERO: Self = GridOptions {
        x: GridAxisOptions {
            min_padding: PhysicalPixels::ZERO,
            min_spacing: PhysicalPixels::ZERO,
            preferred_padding: PhysicalPixels::ZERO,
            preferred_spacing: PhysicalPixels::ZERO,
            border_collapse: PhysicalPixels::ZERO,
            alignment: Alignment::Start,
        },
        y: GridAxisOptions {
            min_padding: PhysicalPixels::ZERO,
            min_spacing: PhysicalPixels::ZERO,
            preferred_padding: PhysicalPixels::ZERO,
            preferred_spacing: PhysicalPixels::ZERO,
            border_collapse: PhysicalPixels::ZERO,
            alignment: Alignment::Start,
        },
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GridAxisOptions {
    pub min_padding: PhysicalPixels,
    pub min_spacing: PhysicalPixels,
    pub preferred_padding: PhysicalPixels,
    pub preferred_spacing: PhysicalPixels,
    pub border_collapse: PhysicalPixels,
    pub alignment: Alignment,
}

fn size_hint(
    items: &[(RangeInclusive<i32>, PhysicalPixels)],
    options: &GridAxisOptions,
    mode: SizeHintMode,
) -> PhysicalPixels {
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
        let current: PhysicalPixels = range
            .clone()
            .map(|pos| max_per_column.get(&pos).copied().unwrap_or(0.ppx()))
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
    max_per_column.values().sum::<PhysicalPixels>()
        + 2 * padding
        + max_per_column.len().saturating_sub(1) as i32 * (spacing - options.border_collapse)
}

pub fn size_hint_x(
    widget: &(impl Widget + ?Sized),
    options: &GridOptions,
    rows_and_columns: &RowsAndColumns,
) -> SizeHints {
    let mut min_items = Vec::new();
    let mut preferred_items = Vec::new();
    let mut all_fixed = true;
    for item in widget.base().children() {
        let Some(pos_in_grid) = rows_and_columns.id_to_x.get(&item.base().id()).cloned() else {
            continue;
        };
        let hints = item.size_hint_x();
        min_items.push((pos_in_grid.clone(), hints.min));
        preferred_items.push((pos_in_grid, hints.preferred));

        let is_fixed = item
            .base()
            .layout_item_options()
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
    widget: &(impl Widget + ?Sized),
    options: &GridOptions,
    size_x: PhysicalPixels,
    rows_and_columns: &RowsAndColumns,
) -> SizeHints {
    let x_layout = x_layout(widget, rows_and_columns, &options.x, size_x);
    let mut min_items = Vec::new();
    let mut preferred_items = Vec::new();
    let mut all_fixed = true;
    for item in widget.base().children() {
        let Some(item_size_x) = x_layout.child_sizes.get(&item.base().id()) else {
            continue;
        };
        let Some(pos_in_grid) = rows_and_columns.id_to_y.get(&item.base().id()).cloned() else {
            continue;
        };
        let hints = item.size_hint_y(*item_size_x);

        min_items.push((pos_in_grid.clone(), hints.min));
        preferred_items.push((pos_in_grid, hints.preferred));

        let is_fixed = item
            .base()
            .layout_item_options()
            .x
            .is_fixed
            .unwrap_or(hints.is_fixed);

        if !is_fixed {
            all_fixed = false;
        }
    }
    SizeHints {
        min: size_hint(&min_items, &options.y, SizeHintMode::Min),
        preferred: size_hint(&preferred_items, &options.y, SizeHintMode::Preferred),
        is_fixed: all_fixed,
    }
}

struct XLayout {
    padding: PhysicalPixels,
    spacing: PhysicalPixels,
    column_sizes: BTreeMap<i32, PhysicalPixels>,
    child_sizes: HashMap<RawWidgetId, PhysicalPixels>,
}

fn x_layout(
    widget: &(impl Widget + ?Sized),
    rows_and_columns: &RowsAndColumns,
    options: &GridAxisOptions,
    size_x: PhysicalPixels,
) -> XLayout {
    let mut hints_per_column = BTreeMap::new();
    for item in widget.base().children() {
        let Some(pos) = rows_and_columns.id_to_x.get(&item.base().id()).cloned() else {
            continue;
        };
        if pos.start() != pos.end() {
            warn!("spanned items are not supported yet");
        }
        let pos = *pos.start();

        let mut hints = item.size_hint_x();
        if let Some(is_fixed) = item.base().layout_item_options().x.is_fixed {
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
    for item in widget.base().children() {
        let Some(pos) = rows_and_columns.id_to_x.get(&item.base().id()).cloned() else {
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
            .base()
            .layout_item_options()
            .x
            .is_fixed
            .unwrap_or_else(|| item.size_hint_x().is_fixed)
        {
            let hint = item.size_hint_x().preferred;
            min(hint, *column_size)
        } else {
            *column_size
        };
        child_sizes.insert(item.base().id(), child_size);
    }
    XLayout {
        padding: output.padding,
        spacing: output.spacing,
        column_sizes,
        child_sizes,
    }
}

#[derive(Default)]
pub struct RowsAndColumns {
    id_to_x: HashMap<RawWidgetId, RangeInclusive<i32>>,
    id_to_y: HashMap<RawWidgetId, RangeInclusive<i32>>,
    x_y_to_id: HashMap<(i32, i32), RawWidgetId>,
}

// TODO: refresh only when relevant things have changed
pub fn assign_rows_and_columns<W: Widget + ?Sized>(widget: &W) -> RowsAndColumns {
    let mut output = RowsAndColumns::default();
    let mut current_x = 0;
    let mut current_y = 0;
    let layout = widget.base().layout();
    // TODO: impl sorting key and sort by (sorting_key, key) here.
    for child in widget.base().children() {
        if child.base().is_window_root() || !child.base().is_self_visible() {
            continue;
        }
        let id = child.base().id();
        let options = child.base().layout_item_options();
        let x;
        let y;
        match layout {
            // TODO: implement "next row" / "next column" API
            super::Layout::VerticalFirst => {
                if options.x.pos_in_grid.is_some() || options.y.pos_in_grid.is_some() {
                    warn!("pos_in_grid is set but ignored because Layout::VerticalFirst is in use");
                }
                x = current_x..=current_x;
                y = current_y..=current_y;
                current_y += 1;
            }
            super::Layout::HorizontalFirst => {
                if options.x.pos_in_grid.is_some() || options.y.pos_in_grid.is_some() {
                    warn!(
                        "pos_in_grid is set but ignored because Layout::HorizontalFirst is in use"
                    );
                }
                x = current_x..=current_x;
                y = current_y..=current_y;
                current_x += 1;
            }
            super::Layout::ExplicitGrid => {
                match (options.x.pos_in_grid.clone(), options.y.pos_in_grid.clone()) {
                    (None, None) => continue,
                    (None, Some(_)) => {
                        warn!("column is set but row is unset, ignoring column");
                        continue;
                    }
                    (Some(_), None) => {
                        warn!("row is set but column is unset, ignoring row");
                        continue;
                    }
                    (Some(x_conf), Some(y_conf)) => {
                        x = x_conf;
                        y = y_conf;
                    }
                }
            }
        }
        match output.x_y_to_id.entry((*x.start(), *y.start())) {
            hash_map::Entry::Occupied(_) => {
                warn!(
                    "assigned same grid pos ({}, {}) to multiple widgets, ignoring duplicate",
                    x.start(),
                    y.start()
                );
                continue;
            }
            hash_map::Entry::Vacant(entry) => {
                entry.insert(id);
            }
        }

        output.id_to_x.insert(id, x);
        output.id_to_y.insert(id, y);
    }

    output
}

pub fn grid_layout<W: Widget + ?Sized>(widget: &mut W, changed_size_hints: &[WidgetAddress]) {
    let Some(geometry) = widget.base().geometry().cloned() else {
        for child in widget.base_mut().children_mut() {
            child.set_geometry(None, changed_size_hints);
        }
        return;
    };
    let options = widget.base().base_style().grid.clone();
    let rows_and_columns = assign_rows_and_columns(widget);
    let x_layout = x_layout(widget, &rows_and_columns, &options.x, geometry.size_x());
    let mut hints_per_row = BTreeMap::new();
    for item in widget.base_mut().children_mut() {
        // TODO: problem with is_self_visible
        let Some(pos) = rows_and_columns.id_to_y.get(&item.base().id()).cloned() else {
            continue;
        };

        if pos.start() != pos.end() {
            warn!("spanned items are not supported yet");
        }
        let Some(item_size_x) = x_layout.child_sizes.get(&item.base().id()) else {
            continue;
        };
        let pos = *pos.start();
        let mut hints = item.size_hint_y(*item_size_x);
        if let Some(is_fixed) = item.base().layout_item_options().y.is_fixed {
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
    let output_y = solve_layout(&layout_items, geometry.size_y(), &options.y);
    let row_sizes: BTreeMap<_, _> = hints_per_row.keys().copied().zip(output_y.sizes).collect();
    let positions_x = positions(
        &x_layout.column_sizes,
        x_layout.padding,
        x_layout.spacing,
        geometry.size_x(),
        options.x.alignment,
    );
    let positions_y = positions(
        &row_sizes,
        output_y.padding,
        output_y.spacing,
        geometry.size_y(),
        options.y.alignment,
    );
    for item in widget.base_mut().children_mut() {
        // if !item.common().is_self_visible {
        //     continue;
        // }

        let Some(pos_x) = rows_and_columns.id_to_x.get(&item.base().id()).cloned() else {
            continue;
        };
        let Some(pos_y) = rows_and_columns.id_to_y.get(&item.base().id()).cloned() else {
            continue;
        };
        let Some(cell_pos_x) = positions_x.get(pos_x.start()) else {
            continue;
        };
        let Some(cell_pos_y) = positions_y.get(pos_y.start()) else {
            warn!("missing item in positions_y");
            continue;
        };
        let Some(size_x) = x_layout.child_sizes.get(&item.base().id()) else {
            warn!("missing item in x_layout.child_sizes");
            continue;
        };
        let Some(row_size) = row_sizes.get(pos_y.start()) else {
            warn!("missing item in row_sizes");
            continue;
        };
        let size_hint_y = item.size_hint_y(*size_x);
        let size_y = if item
            .base()
            .layout_item_options()
            .y
            .is_fixed
            .unwrap_or(size_hint_y.is_fixed)
        {
            min(*row_size, size_hint_y.preferred)
        } else {
            *row_size
        };
        item.set_geometry(
            Some(WidgetGeometry::new(
                &geometry,
                Rect::from_xywh(*cell_pos_x, *cell_pos_y, *size_x, size_y),
            )),
            changed_size_hints,
        );
    }
}

fn positions(
    sizes: &BTreeMap<i32, PhysicalPixels>,
    padding: PhysicalPixels,
    spacing: PhysicalPixels,
    total_available: PhysicalPixels,
    alignment: Alignment,
) -> BTreeMap<i32, PhysicalPixels> {
    let mut pos = padding;
    let total_taken: PhysicalPixels = sizes.values().sum();
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
