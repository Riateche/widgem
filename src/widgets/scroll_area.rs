use std::cmp::max;

use anyhow::Result;
use salvation_macros::impl_with;

use crate::{
    event::LayoutEvent,
    layout::{SizeHintMode, FALLBACK_SIZE_HINT},
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
        common.add_child(ScrollBar::new().boxed(), Default::default());
        common.add_child(
            ScrollBar::new().with_axis(Axis::Y).boxed(),
            Default::default(),
        );
        let mut this = Self { common };

        // let slider_pressed = this.callback(Self::slider_pressed);
        // let slider_moved = this.callback(Self::slider_moved);

        this
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

    fn size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let xscroll_x = self.common.children[0].widget.cached_size_hint_x(mode);
        let yscroll_x = self.common.children[1].widget.cached_size_hint_x(mode);
        let content_x = if let Some(child) = self.common.children.get_mut(2) {
            child.widget.cached_size_hint_x(mode)
        } else {
            FALLBACK_SIZE_HINT
        };
        Ok(max(yscroll_x, xscroll_x + content_x))

        // let hints = self.size_hints();
        // match self.axis {
        //     Axis::X => Ok(SizeHint {
        //         min: hints.x0.min + hints.x1.min + hints.x2.min + 40,
        //         preferred: hints.x0.preferred + hints.x1.preferred + hints.x2.preferred + 80,
        //         is_fixed: false,
        //     }),
        //     Axis::Y => todo!(),
        // }
    }

    fn size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        // let yscroll_x = self.common.children[1].widget.cached_size_hint_x();
        // let yscroll_size_x = min(size_x, yscroll_x.
        // let hints = self.size_hints();
        // match self.axis {
        //     Axis::X => Ok(SizeHint {
        //         min: max(hints.y0.min, max(hints.y1.min, hints.y2.min)),
        //         preferred: max(
        //             hints.y0.preferred,
        //             max(hints.y1.preferred, hints.y2.preferred),
        //         ),
        //         is_fixed: true,
        //     }),
        //     Axis::Y => todo!(),
        // }
        todo!()
    }

    fn is_size_hint_x_fixed(&mut self) -> bool {
        false
    }
    fn is_size_hint_y_fixed(&mut self) -> bool {
        false
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let Some(size) = self.common.size() else {
            return Ok(());
        };
        // let hints = self.size_hints();
        // match self.axis {
        //     Axis::X => {
        //         self.common.set_child_rect(
        //             0,
        //             Some(Rect::from_xywh(
        //                 0,
        //                 0,
        //                 hints.x0.preferred,
        //                 hints.y0.preferred,
        //             )),
        //         )?;
        //         self.starting_slider_rect = Rect::from_xywh(
        //             hints.x0.preferred,
        //             0,
        //             hints.x1.preferred,
        //             hints.y1.preferred,
        //         );
        //         let button2_rect = Rect::from_xywh(
        //             size.x - hints.x2.preferred,
        //             0,
        //             hints.x2.preferred,
        //             hints.y2.preferred,
        //         );
        //         self.max_slider_pos =
        //             button2_rect.top_left.x - self.starting_slider_rect.bottom_right().x;
        //         self.current_slider_pos = self.value_to_slider_pos();
        //         self.common.set_child_rect(
        //             1,
        //             Some(
        //                 self.starting_slider_rect
        //                     .translate(Point::new(self.current_slider_pos, 0)),
        //             ),
        //         )?;
        //         self.common.set_child_rect(2, Some(button2_rect))?;
        //     }
        //     Axis::Y => todo!(),
        // }
        Ok(())
    }
}

struct SizeHints {
    xscroll_x: i32,
    yscroll_x: i32,
    content_x: i32,
    xscroll_y: i32,
    yscroll_y: i32,
    content_y: i32,
}
