use std::cmp::max;

use crate::{
    layout::SizeHint,
    types::{Axis, Rect},
};
use anyhow::Result;

use super::{button::Button, Widget, WidgetCommon, WidgetExt};

pub struct ScrollBar {
    common: WidgetCommon,
    axis: Axis,
}

impl ScrollBar {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new();
        // TODO: icons, localized name
        common.add_child(0, Button::new("<").boxed());
        common.add_child(1, Button::new("|||").boxed());
        common.add_child(2, Button::new(">").boxed());
        Self {
            common,
            axis: Axis::X,
        }
    }

    fn size_hints(&mut self) -> SizeHints {
        let hint0_x = self.common.children[0].widget.cached_size_hint_x();
        let hint1_x = self.common.children[1].widget.cached_size_hint_x();
        let hint2_x = self.common.children[2].widget.cached_size_hint_x();

        let hint0_y = self.common.children[0]
            .widget
            .cached_size_hint_y(hint0_x.preferred);
        let hint1_y = self.common.children[1]
            .widget
            .cached_size_hint_y(hint1_x.preferred);
        let hint2_y = self.common.children[2]
            .widget
            .cached_size_hint_y(hint2_x.preferred);
        SizeHints {
            x0: hint0_x,
            x1: hint1_x,
            x2: hint2_x,
            y0: hint0_y,
            y1: hint1_y,
            y2: hint2_y,
        }
    }
}

impl Default for ScrollBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ScrollBar {
    fn common(&self) -> &super::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut super::WidgetCommon {
        &mut self.common
    }

    fn size_hint_x(&mut self) -> Result<SizeHint> {
        let hints = self.size_hints();
        match self.axis {
            Axis::X => Ok(SizeHint {
                min: hints.x0.min + hints.x1.min + hints.x2.min + 40,
                preferred: hints.x0.preferred + hints.x1.preferred + hints.x2.preferred + 80,
                is_fixed: false,
            }),
            Axis::Y => todo!(),
        }
    }

    fn size_hint_y(&mut self, _size_x: i32) -> Result<SizeHint> {
        let hints = self.size_hints();
        match self.axis {
            Axis::X => Ok(SizeHint {
                min: max(hints.y0.min, max(hints.y1.min, hints.y2.min)),
                preferred: max(
                    hints.y0.preferred,
                    max(hints.y1.preferred, hints.y2.preferred),
                ),
                is_fixed: true,
            }),
            Axis::Y => todo!(),
        }
    }

    fn layout(&mut self) -> Result<Vec<Option<Rect>>> {
        let size = self.common.size_or_err()?;
        let hints = self.size_hints();
        match self.axis {
            Axis::X => Ok(vec![
                Some(Rect::from_xywh(
                    0,
                    0,
                    hints.x0.preferred,
                    hints.y0.preferred,
                )),
                Some(Rect::from_xywh(
                    hints.x0.preferred,
                    0,
                    hints.x1.preferred,
                    hints.y1.preferred,
                )),
                Some(Rect::from_xywh(
                    size.x - hints.x2.preferred,
                    0,
                    hints.x2.preferred,
                    hints.y2.preferred,
                )),
            ]),
            Axis::Y => todo!(),
        }
    }
}

struct SizeHints {
    x0: SizeHint,
    x1: SizeHint,
    x2: SizeHint,
    y0: SizeHint,
    y1: SizeHint,
    y2: SizeHint,
}

/*struct ScrollBarSlider {
    common: WidgetCommon,
}

impl ScrollBarSlider {
    pub fn new() -> Self {
        Self {
            common: WidgetCommon::new(),
        }
    }
}

impl Widget for ScrollBarSlider {
    fn common(&self) -> &super::WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut super::WidgetCommon {
        &mut self.common
    }

    fn size_hint_x(&mut self) -> Result<SizeHint> {
        // TODO: from style
        Ok(SizeHint {
            min: 20,
            preferred: 40,
            is_fixed: true,
        })
    }

    fn size_hint_y(&mut self, _size_x: i32) -> Result<SizeHint> {
        // TODO: from style
        Ok(SizeHint {
            min: 20,
            preferred: 40,
            is_fixed: false,
        })
    }
}
*/
