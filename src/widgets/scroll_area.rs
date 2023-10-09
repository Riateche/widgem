use anyhow::Result;
use salvation_macros::impl_with;

use crate::{
    event::LayoutEvent,
    layout::{
        grid::{self, GridAxisOptions, GridOptions},
        LayoutItemOptions, SizeHintMode,
    },
    types::Axis,
};

use super::{scroll_bar::ScrollBar, Widget, WidgetCommon, WidgetExt};

pub struct ScrollArea {
    common: WidgetCommon,
}

#[impl_with]
impl ScrollArea {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new();
        // TODO: icons, localized name
        common.add_child(
            ScrollBar::new().boxed(),
            LayoutItemOptions::from_pos_in_grid(0, 1),
        );
        common.add_child(
            ScrollBar::new().with_axis(Axis::Y).boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 0),
        );
        common.add_child(
            Viewport::new().boxed(),
            LayoutItemOptions::from_pos_in_grid(0, 0),
        );
        // let mut this = Self { common };
        // let slider_pressed = this.callback(Self::slider_pressed);
        // let slider_moved = this.callback(Self::slider_moved);
        // this
        Self { common }
    }

    // pub fn on_value_changed(&mut self, callback: Callback<i32>) {
    //     self.value_changed = Some(callback);
    // }

    // fn size_hints(&mut self) -> SizeHints {
    //     let xscroll_x = self.common.children[0].widget.cached_size_hint_x();
    //     let yscroll_x = self.common.children[1].widget.cached_size_hint_x();
    //     let content_x = if let Some(child) = self.common.children.get(2) {
    //         widget.cached_size_hint_x()
    //     } else {
    //         SizeHint::new_fallback()
    //     };

    //     let xscroll_y = self.common.children[0]
    //         .widget
    //         .cached_size_hint_y(xscroll_x.preferred);
    //     let yscroll_y = self.common.children[1]
    //         .widget
    //         .cached_size_hint_y(yscroll_x.preferred);
    //     let content_y = self.common.children[2]
    //         .widget
    //         .cached_size_hint_y(content_x.preferred);
    //     SizeHints {
    //         xscroll_x,
    //         yscroll_x,
    //         content_x,
    //         xscroll_y: xscroll_y,
    //         yscroll_y,
    //         content_y,
    //     }
    // }
    fn grid_options(&self) -> GridOptions {
        GridOptions {
            x: GridAxisOptions {
                min_padding: 0,
                min_spacing: 0,
                preferred_padding: 0,
                preferred_spacing: 0,
            },
            y: GridAxisOptions {
                min_padding: 0,
                min_spacing: 0,
                preferred_padding: 0,
                preferred_spacing: 0,
            },
        }
    }
}

impl Default for ScrollArea {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ScrollArea {
    fn common(&self) -> &super::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut super::WidgetCommon {
        &mut self.common
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let options = self.grid_options();
        let size = self.common.size_or_err()?;
        let rects = grid::layout(&mut self.common.children, &options, size)?;
        self.common.set_child_rects(&rects)?;
        Ok(())
    }

    fn size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let options = self.grid_options();
        grid::size_hint_x(&mut self.common.children, &options, mode)
    }
    fn is_size_hint_x_fixed(&mut self) -> bool {
        let options = self.grid_options();
        grid::is_size_hint_x_fixed(&mut self.common.children, &options)
    }
    fn is_size_hint_y_fixed(&mut self) -> bool {
        let options = self.grid_options();
        grid::is_size_hint_y_fixed(&mut self.common.children, &options)
    }
    fn size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        let options = self.grid_options();
        grid::size_hint_y(&mut self.common.children, &options, size_x, mode)
    }
}

// TODO: public type for empty widget?
struct Viewport {
    common: WidgetCommon,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            common: WidgetCommon::new(),
        }
    }
}

impl Widget for Viewport {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }
    fn size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }
    fn is_size_hint_x_fixed(&mut self) -> bool {
        false
    }
    fn is_size_hint_y_fixed(&mut self) -> bool {
        false
    }
}
