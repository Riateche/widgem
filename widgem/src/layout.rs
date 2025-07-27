use {
    crate::{
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

/// Layout strategy.
///
/// Set the layout strategy for a widget using [WidgetBase::set_layout](crate::WidgetBase::set_layout)
/// or [WidgetExt::set_layout].
///
/// Layout strategy determines how child widgets are positioned within the widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Layout {
    /// Child widgets are layed out from top to bottom. This is the default for most widgets.
    #[default]
    VerticalFirst,
    // TODO: it should depend on writing direction
    /// Child widgets are layed out from left to right.
    HorizontalFirst,
    /// Child widgets are positioned in a grid according to the row and column settings
    /// of each child.
    ExplicitGrid,
    // TODO: other layout types? disabled variant?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SizeHint {
    min: PhysicalPixels,
    preferred: PhysicalPixels,
    is_fixed: bool,
}

impl SizeHint {
    pub fn new(min: PhysicalPixels, preferred: PhysicalPixels, is_fixed: bool) -> Self {
        Self {
            min,
            preferred,
            is_fixed,
        }
    }

    pub fn new_fixed(min: PhysicalPixels, preferred: PhysicalPixels) -> Self {
        Self::new(min, preferred, true)
    }

    pub fn new_expanding(min: PhysicalPixels, preferred: PhysicalPixels) -> Self {
        Self::new(min, preferred, false)
    }

    pub fn min(&self) -> PhysicalPixels {
        self.min
    }

    pub fn set_min(&mut self, min: PhysicalPixels) {
        self.min = min;
    }

    pub fn preferred(&self) -> PhysicalPixels {
        self.preferred
    }

    pub fn set_preferred(&mut self, preferred: PhysicalPixels) {
        self.preferred = preferred;
    }

    pub fn is_fixed(&self) -> bool {
        self.is_fixed
    }

    pub fn set_fixed(&mut self, is_fixed: bool) {
        self.is_fixed = is_fixed;
    }
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

pub(crate) const FALLBACK_SIZE_HINT: i32 = 48;
pub(crate) const FALLBACK_SIZE_HINTS: SizeHint = SizeHint {
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
    x: LayoutItemAxisOptions,
    y: LayoutItemAxisOptions,
}

impl LayoutItemOptions {
    pub fn x(&self) -> &LayoutItemAxisOptions {
        &self.x
    }

    pub fn y(&self) -> &LayoutItemAxisOptions {
        &self.y
    }

    pub fn set_x(&mut self, x: LayoutItemAxisOptions) {
        self.x = x;
    }

    pub fn set_y(&mut self, y: LayoutItemAxisOptions) {
        self.y = y;
    }

    pub fn set_grid_cell(&mut self, x: i32, y: i32) {
        self.x.grid_cell = Some(x..=x);
        self.y.grid_cell = Some(y..=y);
    }

    pub fn unset_grid_cell(&mut self) {
        self.x.grid_cell = None;
        self.y.grid_cell = None;
    }

    pub fn set_x_alignment(&mut self, alignment: Option<Alignment>) {
        self.x.alignment = alignment;
    }

    pub fn set_y_alignment(&mut self, alignment: Option<Alignment>) {
        self.y.alignment = alignment;
    }

    pub fn set_x_fixed(&mut self, is_fixed: Option<bool>) {
        self.x.is_fixed = is_fixed;
    }

    pub fn set_y_fixed(&mut self, is_fixed: Option<bool>) {
        self.y.is_fixed = is_fixed;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct LayoutItemAxisOptions {
    // row or column
    grid_cell: Option<RangeInclusive<i32>>,
    alignment: Option<Alignment>,
    is_fixed: Option<bool>,
    // TODO: alignment, priority, stretch, etc.
}

impl LayoutItemAxisOptions {
    pub fn grid_cell(&self) -> Option<RangeInclusive<i32>> {
        self.grid_cell.clone()
    }

    pub fn alignment(&self) -> Option<Alignment> {
        self.alignment
    }

    pub fn is_fixed(&self) -> Option<bool> {
        self.is_fixed
    }

    pub fn set_grid_cell(&mut self, pos_in_grid: Option<RangeInclusive<i32>>) {
        self.grid_cell = pos_in_grid;
    }

    pub fn set_alignment(&mut self, alignment: Option<Alignment>) {
        self.alignment = alignment;
    }

    pub fn set_is_fixed(&mut self, is_fixed: Option<bool>) {
        self.is_fixed = is_fixed;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Alignment {
    Start,
    Middle,
    End,
    // TODO: justify?
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
    pub(crate) size_hints: SizeHint,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GridOptions {
    pub(crate) x: GridAxisOptions,
    pub(crate) y: GridAxisOptions,
}

impl GridOptions {
    #[allow(dead_code)]
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
pub(crate) struct GridAxisOptions {
    pub(crate) min_padding: PhysicalPixels,
    pub(crate) min_spacing: PhysicalPixels,
    pub(crate) preferred_padding: PhysicalPixels,
    pub(crate) preferred_spacing: PhysicalPixels,
    pub(crate) border_collapse: PhysicalPixels,
    pub(crate) alignment: Alignment,
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

pub fn default_size_hint_x(widget: &(impl Widget + ?Sized)) -> SizeHint {
    let options = widget.base().base_style().grid.clone();
    let rows_and_columns = assign_rows_and_columns(widget);
    size_hint_x(widget, &options, &rows_and_columns)
}

pub(crate) fn size_hint_x(
    widget: &(impl Widget + ?Sized),
    options: &GridOptions,
    rows_and_columns: &RowsAndColumns,
) -> SizeHint {
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
    SizeHint {
        min: size_hint(&min_items, &options.x, SizeHintMode::Min),
        preferred: size_hint(&preferred_items, &options.x, SizeHintMode::Preferred),
        is_fixed: all_fixed,
    }
}

pub fn default_size_hint_y(widget: &(impl Widget + ?Sized), size_x: PhysicalPixels) -> SizeHint {
    let options = widget.base().base_style().grid.clone();
    let rows_and_columns = assign_rows_and_columns(widget);
    size_hint_y(widget, &options, size_x, &rows_and_columns)
}

pub(crate) fn size_hint_y(
    widget: &(impl Widget + ?Sized),
    options: &GridOptions,
    size_x: PhysicalPixels,
    rows_and_columns: &RowsAndColumns,
) -> SizeHint {
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
    SizeHint {
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
        .map(|hints| LayoutItem { size_hints: *hints })
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
pub(crate) struct RowsAndColumns {
    id_to_x: HashMap<RawWidgetId, RangeInclusive<i32>>,
    id_to_y: HashMap<RawWidgetId, RangeInclusive<i32>>,
    x_y_to_id: HashMap<(i32, i32), RawWidgetId>,
}

// TODO: refresh only when relevant things have changed
pub(crate) fn assign_rows_and_columns<W: Widget + ?Sized>(widget: &W) -> RowsAndColumns {
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
            Layout::VerticalFirst => {
                if options.x.grid_cell.is_some() || options.y.grid_cell.is_some() {
                    warn!("pos_in_grid is set but ignored because Layout::VerticalFirst is in use");
                }
                x = current_x..=current_x;
                y = current_y..=current_y;
                current_y += 1;
            }
            Layout::HorizontalFirst => {
                if options.x.grid_cell.is_some() || options.y.grid_cell.is_some() {
                    warn!(
                        "pos_in_grid is set but ignored because Layout::HorizontalFirst is in use"
                    );
                }
                x = current_x..=current_x;
                y = current_y..=current_y;
                current_x += 1;
            }
            Layout::ExplicitGrid => {
                match (options.x.grid_cell.clone(), options.y.grid_cell.clone()) {
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

// TODO: remove explicit changed_size_hints, pass in global context?
pub fn default_layout<W: Widget + ?Sized>(widget: &mut W, changed_size_hints: &[WidgetAddress]) {
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
        .map(|hints| LayoutItem { size_hints: *hints })
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
