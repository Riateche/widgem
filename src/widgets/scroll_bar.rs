use std::cmp::max;

use crate::{
    callback::Callback,
    event::{Event, LayoutEvent},
    layout::SizeHintMode,
    types::{Axis, Point, Rect},
};
use anyhow::Result;
use log::warn;
use salvation_macros::impl_with;
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

#[impl_with]
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

    pub fn set_axis(&mut self, axis: Axis) {
        self.axis = axis;
        self.common.size_hint_changed();
    }

    pub fn on_value_changed(&mut self, callback: Callback<i32>) {
        self.value_changed = Some(callback);
    }

    fn size_hints(&mut self, mode: SizeHintMode) -> SizeHints {
        let hint0_x = self.common.children[0].widget.cached_size_hint_x(mode);
        let hint1_x = self.common.children[1].widget.cached_size_hint_x(mode);
        let hint2_x = self.common.children[2].widget.cached_size_hint_x(mode);

        let hint0_y = self.common.children[0]
            .widget
            .cached_size_hint_y(hint0_x, mode);
        let hint1_y = self.common.children[1]
            .widget
            .cached_size_hint_y(hint1_x, mode);
        let hint2_y = self.common.children[2]
            .widget
            .cached_size_hint_y(hint2_x, mode);
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
            match self.axis {
                Axis::X => {
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
                Axis::Y => {
                    // TODO: deduplicate
                    let new_pos = start_slider_pos - start_mouse_pos.y + pos_in_window.y;
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
                                .translate(Point::new(0, self.current_slider_pos)),
                        ),
                    )?;
                }
            }
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

    fn size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let hints = self.size_hints(mode);
        match self.axis {
            Axis::X => Ok(hints.x0 + hints.x1 + hints.x2 + 40),
            Axis::Y => Ok(max(hints.x0, max(hints.x1, hints.x2))),
        }
    }

    fn size_hint_y(&mut self, _size_x: i32, mode: SizeHintMode) -> Result<i32> {
        let hints = self.size_hints(mode);
        match self.axis {
            Axis::X => Ok(max(hints.y0, max(hints.y1, hints.y2))),
            Axis::Y => Ok(hints.y0 + hints.y1 + hints.y2 + 40),
        }
    }

    fn is_size_hint_x_fixed(&mut self) -> bool {
        self.axis == Axis::Y
    }
    fn is_size_hint_y_fixed(&mut self) -> bool {
        self.axis == Axis::X
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let Some(size) = self.common.size() else {
            return Ok(());
        };
        let hints = self.size_hints(SizeHintMode::Preferred);
        match self.axis {
            Axis::X => {
                self.common
                    .set_child_rect(0, Some(Rect::from_xywh(0, 0, hints.x0, hints.y0)))?;
                self.starting_slider_rect = Rect::from_xywh(hints.x0, 0, hints.x1, hints.y1);
                let button2_rect = Rect::from_xywh(size.x - hints.x2, 0, hints.x2, hints.y2);
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
            Axis::Y => {
                self.common
                    .set_child_rect(0, Some(Rect::from_xywh(0, 0, hints.x0, hints.y0)))?;
                self.starting_slider_rect = Rect::from_xywh(0, hints.y0, hints.x1, hints.y1);
                let button2_rect = Rect::from_xywh(0, size.y - hints.y2, hints.x2, hints.y2);
                self.max_slider_pos =
                    button2_rect.top_left.y - self.starting_slider_rect.bottom_right().y;
                self.current_slider_pos = self.value_to_slider_pos();
                self.common.set_child_rect(
                    1,
                    Some(
                        self.starting_slider_rect
                            .translate(Point::new(0, self.current_slider_pos)),
                    ),
                )?;
                self.common.set_child_rect(2, Some(button2_rect))?;
            }
        }
        Ok(())
    }
}

struct SizeHints {
    x0: i32,
    x1: i32,
    x2: i32,
    y0: i32,
    y1: i32,
    y2: i32,
}
