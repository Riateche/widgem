use crate::{
    callback::Callback,
    event::{Event, LayoutEvent},
    layout::{
        grid::{self, GridAxisOptions, GridOptions},
        LayoutItemOptions, SizeHintMode,
    },
    types::{Axis, Point, Rect},
};
use anyhow::Result;
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
        common.add_child(
            Button::new("<").boxed(),
            LayoutItemOptions::from_pos_in_grid(0, 0),
        );
        let mut slider = Button::new("|||");

        slider.common_mut().event_filter = Some(Box::new(|event| {
            println!("filtered event: {event:?}");
            Ok(false)
        }));
        common.add_child(
            Pager::new().boxed(),
            LayoutItemOptions::from_pos_in_grid(1, 0),
        );
        common.add_child(
            Button::new(">").boxed(),
            LayoutItemOptions::from_pos_in_grid(2, 0),
        );
        common.add_child(slider.boxed(), LayoutItemOptions::default());
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
}

impl Pager {
    pub fn new() -> Self {
        Self {
            common: WidgetCommon::new(),
        }
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
