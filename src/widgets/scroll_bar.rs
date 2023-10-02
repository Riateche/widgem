use std::cmp::max;

use crate::{
    callback::Callback,
    event::{Event, LayoutEvent},
    layout::SizeHint,
    types::{Axis, Point, Rect},
};
use anyhow::Result;
use log::warn;
use winit::event::{ElementState, MouseButton};

use super::{button::Button, Widget, WidgetCommon, WidgetExt};

pub struct ScrollBar {
    common: WidgetCommon,
    axis: Axis,
    current_slider_pos: i32,
    max_slider_pos: i32,
    starting_slider_rect: Rect,
    min_value: i32,
    max_value: i32,
    current_value: i32,
    slider_grab_pos: Option<(Point, i32)>,
    value_changed: Option<Callback<i32>>,
}

impl ScrollBar {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new();
        // TODO: icons, localized name
        common.add_child(0, Button::new("<").boxed());
        let mut slider = Button::new("|||");

        slider.common_mut().event_filter = Some(Box::new(|event| {
            println!("filtered event: {event:?}");
            Ok(false)
        }));
        common.add_child(1, slider.boxed());
        common.add_child(2, Button::new(">").boxed());
        let mut this = Self {
            common,
            axis: Axis::X,
            current_slider_pos: 0,
            max_slider_pos: 0,
            starting_slider_rect: Rect::default(),
            min_value: 0,
            max_value: 100,
            current_value: 20, // TODO
            slider_grab_pos: None,
            value_changed: None,
        };

        let slider_pressed = this.callback(Self::slider_pressed);
        let slider_moved = this.callback(Self::slider_moved);
        this.common.children[1].widget.common_mut().event_filter = Some(Box::new(move |event| {
            match event {
                Event::MouseInput(e) => {
                    if e.button() == MouseButton::Left {
                        slider_pressed.invoke((e.pos_in_window(), e.state()));
                    }
                }
                Event::MouseMove(e) => slider_moved.invoke(e.pos_in_window),
                _ => {}
            }
            Ok(false)
        }));

        this
    }

    pub fn on_value_changed(&mut self, callback: Callback<i32>) {
        self.value_changed = Some(callback);
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

    fn value_to_slider_pos(&self) -> i32 {
        if self.min_value > self.max_value {
            warn!("invalid scroll bar range");
            return 0;
        }
        if self.min_value == self.max_value {
            return 0;
        }
        let pos = (self.current_value - self.min_value) as f32
            / (self.max_value - self.min_value) as f32
            * self.max_slider_pos as f32;
        pos.round() as i32
    }

    fn slider_pressed(&mut self, (pos_in_window, state): (Point, ElementState)) -> Result<()> {
        match state {
            ElementState::Pressed => {
                self.slider_grab_pos = Some((pos_in_window, self.current_slider_pos));
            }
            ElementState::Released => {
                self.slider_grab_pos = None;
            }
        }
        Ok(())
    }

    fn slider_moved(&mut self, pos_in_window: Point) -> Result<()> {
        if let Some((start_mouse_pos, start_slider_pos)) = &self.slider_grab_pos {
            let new_pos = start_slider_pos - start_mouse_pos.x + pos_in_window.x;
            self.current_slider_pos = new_pos.clamp(0, self.max_slider_pos);
            self.current_value = if self.max_slider_pos == 0 {
                self.min_value
            } else {
                ((self.current_slider_pos as f32) / (self.max_slider_pos as f32)
                    * (self.max_value - self.min_value) as f32)
                    .round() as i32
                    + self.min_value
            };
            if let Some(value_changed) = &self.value_changed {
                value_changed.invoke(self.current_value);
            }
            self.common.set_child_rect(
                1,
                Some(
                    self.starting_slider_rect
                        .translate(Point::new(self.current_slider_pos, 0)),
                ),
            )?;
        }
        Ok(())
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

    fn on_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let Some(size) = self.common.size() else {
            return Ok(());
        };
        let hints = self.size_hints();
        match self.axis {
            Axis::X => {
                self.common.set_child_rect(
                    0,
                    Some(Rect::from_xywh(
                        0,
                        0,
                        hints.x0.preferred,
                        hints.y0.preferred,
                    )),
                )?;
                self.starting_slider_rect = Rect::from_xywh(
                    hints.x0.preferred,
                    0,
                    hints.x1.preferred,
                    hints.y1.preferred,
                );
                let button2_rect = Rect::from_xywh(
                    size.x - hints.x2.preferred,
                    0,
                    hints.x2.preferred,
                    hints.y2.preferred,
                );
                self.max_slider_pos =
                    button2_rect.top_left.x - self.starting_slider_rect.bottom_right().x;
                self.current_slider_pos = self.value_to_slider_pos();
                self.common.set_child_rect(
                    1,
                    Some(
                        self.starting_slider_rect
                            .translate(Point::new(self.current_slider_pos, 0)),
                    ),
                )?;
                self.common.set_child_rect(2, Some(button2_rect))?;
            }
            Axis::Y => todo!(),
        }
        Ok(())
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
