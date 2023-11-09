use std::ops::RangeInclusive;

use crate::{
    callback::Callback,
    event::{Event, LayoutEvent},
    layout::{
        grid::{self, GridAxisOptions, GridOptions},
        LayoutItemOptions, SizeHintMode,
    },
    types::{Axis, Point, Rect, Size},
};
use anyhow::Result;
use log::warn;
use salvation_macros::impl_with;
use winit::event::{ElementState, MouseButton};

use super::{
    button::{self, Button},
    Widget, WidgetCommon, WidgetExt,
};

pub struct ScrollBar {
    common: WidgetCommon,
    axis: Axis,
    current_slider_pos: i32,
    max_slider_pos: i32,
    starting_slider_rect: Rect,
    value_range: RangeInclusive<i32>,
    current_value: i32,
    slider_grab_pos: Option<(Point, i32)>,
    value_changed: Option<Callback<i32>>,
}

const SLIDER_LENGTH: i32 = 60;

#[impl_with]
impl ScrollBar {
    pub fn new() -> Self {
        let mut common = WidgetCommon::new();
        // TODO: icons, localized name
        common.add_child(
            Button::new("<")
                .with_role(button::Role1::ScrollLeft)
                .boxed(),
            LayoutItemOptions::from_pos_in_grid(0, 0),
        );
        let slider = Button::new("|||");

        let axis = Axis::X;
        common.add_child(
            Pager::new(axis).boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 0),
        );
        common.add_child(
            Button::new(">").boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 0),
        );
        common.add_child(slider.boxed(), LayoutItemOptions::default());
        let mut this = Self {
            common,
            axis,
            current_slider_pos: 0,
            max_slider_pos: 0,
            starting_slider_rect: Rect::default(),
            value_range: 0..=100,
            current_value: 20, // TODO
            slider_grab_pos: None,
            value_changed: None,
        };

        let slider_pressed = this.callback(Self::slider_pressed);
        let slider_moved = this.callback(Self::slider_moved);
        this.common.children[3].widget.common_mut().event_filter = Some(Box::new(move |event| {
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
        match axis {
            Axis::X => {
                self.common
                    .set_child_options(1, LayoutItemOptions::from_pos_in_grid(1, 0))
                    .unwrap();
                self.common
                    .set_child_options(2, LayoutItemOptions::from_pos_in_grid(2, 0))
                    .unwrap();
            }
            Axis::Y => {
                self.common
                    .set_child_options(1, LayoutItemOptions::from_pos_in_grid(0, 1))
                    .unwrap();
                self.common
                    .set_child_options(2, LayoutItemOptions::from_pos_in_grid(0, 2))
                    .unwrap();
            }
        }
        self.common.children[1]
            .widget
            .downcast_mut::<Pager>()
            .unwrap()
            .set_axis(axis);
    }

    pub fn on_value_changed(&mut self, callback: Callback<i32>) {
        self.value_changed = Some(callback);
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
                        *self.value_range.start()
                    } else {
                        ((self.current_slider_pos as f32) / (self.max_slider_pos as f32)
                            * (*self.value_range.end() - *self.value_range.start()) as f32)
                            .round() as i32
                            + self.value_range.start()
                    };
                    if let Some(value_changed) = &self.value_changed {
                        value_changed.invoke(self.current_value);
                    }
                    self.common.set_child_rect(
                        3,
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
                        *self.value_range.start()
                    } else {
                        ((self.current_slider_pos as f32) / (self.max_slider_pos as f32)
                            * (*self.value_range.end() - *self.value_range.start()) as f32)
                            .round() as i32
                            + *self.value_range.start()
                    };
                    if let Some(value_changed) = &self.value_changed {
                        value_changed.invoke(self.current_value);
                    }
                    self.common.set_child_rect(
                        3,
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

    pub fn set_value_range(&mut self, mut range: RangeInclusive<i32>) {
        if range.end() < range.start() {
            warn!("invalid scroll bar range");
            range = *range.start()..=*range.start();
        }
        if self.value_range == range {
            return;
        }
        self.value_range = range;
        if self.current_value < *self.value_range.start()
            || self.current_value > *self.value_range.end()
        {
            self.set_value(
                self.current_value
                    .clamp(*self.value_range.start(), *self.value_range.end()),
            );
        } else {
            self.update_slider_pos();
        }
    }

    pub fn set_value(&mut self, mut value: i32) {
        if value < *self.value_range.start() || value > *self.value_range.end() {
            warn!("scroll bar value out of bounds");
            value = value.clamp(*self.value_range.start(), *self.value_range.end());
        }
        if self.current_value == value {
            return;
        }
        self.current_value = value;
        if let Some(value_changed) = &self.value_changed {
            value_changed.invoke(self.current_value);
        }
        self.update_slider_pos();
    }

    fn update_slider_pos(&mut self) {
        self.current_slider_pos = self.value_to_slider_pos();
        let shift = match self.axis {
            Axis::X => Point::new(self.current_slider_pos, 0),
            Axis::Y => Point::new(0, self.current_slider_pos),
        };
        self.common
            .set_child_rect(3, Some(self.starting_slider_rect.translate(shift)))
            .unwrap();
    }

    pub fn value(&self) -> i32 {
        self.current_value
    }

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

    fn value_to_slider_pos(&self) -> i32 {
        if *self.value_range.start() > *self.value_range.end() {
            warn!("invalid scroll bar range");
            return 0;
        }
        if *self.value_range.start() == *self.value_range.end() {
            return 0;
        }
        let pos = (self.current_value - *self.value_range.start()) as f32
            / (*self.value_range.end() - *self.value_range.start()) as f32
            * self.max_slider_pos as f32;
        pos.round() as i32
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

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        let options = self.grid_options();
        let size = self.common.size_or_err()?;
        let rects = grid::layout(&mut self.common.children, &options, size)?;
        self.common.set_child_rects(&rects)?;
        let pager_rect = rects.get(&1).unwrap();

        //let slider_size_x = self.common.children[3].widget.cached_size_hint_x(SizeHintMode::Preferred)
        match self.axis {
            Axis::X => {
                self.starting_slider_rect = Rect::from_pos_size(
                    pager_rect.top_left,
                    Size::new(SLIDER_LENGTH, pager_rect.size.y),
                );
                self.max_slider_pos =
                    pager_rect.bottom_right().x - self.starting_slider_rect.bottom_right().x;
            }
            Axis::Y => {
                self.starting_slider_rect = Rect::from_pos_size(
                    pager_rect.top_left,
                    Size::new(pager_rect.size.x, SLIDER_LENGTH),
                );
                self.max_slider_pos =
                    pager_rect.bottom_right().y - self.starting_slider_rect.bottom_right().y;
            }
        }
        self.update_slider_pos();
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

struct Pager {
    common: WidgetCommon,
    axis: Axis,
}

impl Pager {
    pub fn new(axis: Axis) -> Self {
        Self {
            common: WidgetCommon::new(),
            axis,
        }
    }

    pub fn set_axis(&mut self, axis: Axis) {
        self.axis = axis;
        self.common.size_hint_changed();
    }
}

impl Widget for Pager {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    // TODO: from theme?
    fn size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        match self.axis {
            Axis::X => Ok(SLIDER_LENGTH * 2),
            Axis::Y => Ok(45),
        }
    }
    fn size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        match self.axis {
            Axis::X => Ok(45),
            Axis::Y => Ok(SLIDER_LENGTH * 2),
        }
    }
    fn is_size_hint_x_fixed(&mut self) -> bool {
        self.axis == Axis::Y
    }
    fn is_size_hint_y_fixed(&mut self) -> bool {
        self.axis == Axis::X
    }
}
