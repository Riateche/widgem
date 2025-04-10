use {
    super::{button::Button, Widget, WidgetCommon, WidgetCommonTyped, WidgetExt},
    crate::{
        callback::Callback,
        event::{
            Event, FocusInEvent, FocusOutEvent, KeyboardInputEvent, LayoutEvent, MouseScrollEvent,
        },
        impl_widget_common,
        layout::{
            grid::{self, GridAxisOptions, GridOptions},
            Alignment, LayoutItemOptions, SizeHintMode,
        },
        system::ReportError,
        types::{Axis, Point, Rect, Size},
    },
    anyhow::Result,
    log::warn,
    ordered_float::NotNan,
    salvation_macros::impl_with,
    std::{
        cmp::{max, min},
        ops::RangeInclusive,
    },
    winit::{
        event::{ElementState, MouseButton},
        keyboard::{Key, NamedKey},
    },
};

pub struct ScrollBar {
    common: WidgetCommon,
    axis: Axis,
    current_grip_pos: i32,
    max_slider_pos: i32,
    grip_size: Size,
    value_range: RangeInclusive<i32>,
    current_value: i32,
    step: i32,
    slider_grab_pos: Option<(Point, i32)>,
    value_changed: Option<Callback<i32>>,
    pager_direction: i32,
    pager_mouse_pos_in_window: Point,
}

mod names {
    pub const SCROLL_LEFT: &str = "scroll left";
    pub const SCROLL_RIGHT: &str = "scroll right";
    pub const SCROLL_UP: &str = "scroll up";
    pub const SCROLL_DOWN: &str = "scroll down";
    pub const SCROLL_GRIP: &str = "scroll grip";
    pub const SCROLL_PAGER: &str = "scroll pager";
}

const INDEX_DECREASE: usize = 0;
const INDEX_PAGER: usize = 1;
const INDEX_INCREASE: usize = 2;

const INDEX_BUTTON_IN_PAGER: usize = 0;
const INDEX_GRIP_IN_PAGER: usize = 1;

#[impl_with]
impl ScrollBar {
    pub fn increase(&mut self) {
        self.set_value(min(*self.value_range().end(), self.value() + self.step));
    }

    pub fn decrease(&mut self) {
        self.set_value(max(*self.value_range().start(), self.value() - self.step));
    }

    pub fn axis(&mut self) -> Axis {
        self.axis
    }

    pub fn set_axis(&mut self, axis: Axis) -> &mut Self {
        if self.axis == axis {
            return self;
        }
        self.axis = axis;
        match axis {
            Axis::X => {
                let decrease = self.common.children[INDEX_DECREASE]
                    .widget
                    .downcast_mut::<Button>()
                    .unwrap();
                decrease.set_text(names::SCROLL_LEFT);
                decrease.add_class("scroll_left");
                decrease.remove_class("scroll_up");

                let increase = self.common.children[INDEX_INCREASE]
                    .widget
                    .downcast_mut::<Button>()
                    .unwrap();
                increase.set_text(names::SCROLL_RIGHT);
                increase.add_class("scroll_right");
                increase.remove_class("scroll_down");

                let grip = self.common.children[INDEX_PAGER]
                    .widget
                    .common_mut()
                    .children[INDEX_GRIP_IN_PAGER]
                    .widget
                    .downcast_mut::<Button>()
                    .unwrap();
                grip.add_class("scroll_grip_x");
                grip.remove_class("scroll_grip_y");

                self.common
                    .set_child_options(INDEX_PAGER, LayoutItemOptions::from_pos_in_grid(1, 0))
                    .unwrap();
                self.common
                    .set_child_options(INDEX_INCREASE, LayoutItemOptions::from_pos_in_grid(2, 0))
                    .unwrap();
            }
            Axis::Y => {
                let decrease = self.common.children[INDEX_DECREASE]
                    .widget
                    .downcast_mut::<Button>()
                    .unwrap();
                decrease.set_text(names::SCROLL_UP);
                decrease.remove_class("scroll_left");
                decrease.add_class("scroll_up");

                let increase = self.common.children[INDEX_INCREASE]
                    .widget
                    .downcast_mut::<Button>()
                    .unwrap();
                increase.set_text(names::SCROLL_DOWN);
                increase.remove_class("scroll_right");
                increase.add_class("scroll_down");

                let grip = self.common.children[INDEX_PAGER]
                    .widget
                    .common_mut()
                    .children[INDEX_GRIP_IN_PAGER]
                    .widget
                    .downcast_mut::<Button>()
                    .unwrap();

                grip.remove_class("scroll_grip_x");
                grip.add_class("scroll_grip_y");

                self.common
                    .set_child_options(INDEX_PAGER, LayoutItemOptions::from_pos_in_grid(0, 1))
                    .unwrap();
                self.common
                    .set_child_options(INDEX_INCREASE, LayoutItemOptions::from_pos_in_grid(0, 2))
                    .unwrap();
            }
        }
        self.common.children[INDEX_PAGER]
            .widget
            .downcast_mut::<Pager>()
            .unwrap()
            .set_axis(axis);
        self
    }

    pub fn on_value_changed(&mut self, callback: Callback<i32>) -> &mut Self {
        self.value_changed = Some(callback);
        self
    }

    fn slider_pressed(&mut self, (pos_in_window, state): (Point, ElementState)) -> Result<()> {
        match state {
            ElementState::Pressed => {
                self.slider_grab_pos = Some((pos_in_window, self.current_grip_pos));
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
                    self.current_grip_pos = new_pos.clamp(0, self.max_slider_pos);
                    let new_value = if self.max_slider_pos == 0 {
                        *self.value_range.start()
                    } else {
                        ((self.current_grip_pos as f32) / (self.max_slider_pos as f32)
                            * (*self.value_range.end() - *self.value_range.start()) as f32)
                            .round() as i32
                            + self.value_range.start()
                    };
                    self.set_value(new_value);
                }
                Axis::Y => {
                    // TODO: deduplicate
                    let new_pos = start_slider_pos - start_mouse_pos.y + pos_in_window.y;
                    self.current_grip_pos = new_pos.clamp(0, self.max_slider_pos);
                    let new_value = if self.max_slider_pos == 0 {
                        *self.value_range.start()
                    } else {
                        ((self.current_grip_pos as f32) / (self.max_slider_pos as f32)
                            * (*self.value_range.end() - *self.value_range.start()) as f32)
                            .round() as i32
                            + *self.value_range.start()
                    };
                    self.set_value(new_value);
                }
            }
        }
        Ok(())
    }

    fn pager_pressed(&mut self, pos_in_window: Point) -> Result<()> {
        let Some(grip_rect_in_window) = self.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .children[INDEX_GRIP_IN_PAGER]
            .widget
            .common_mut()
            .rect_in_window
        else {
            return Ok(());
        };
        self.pager_direction = match self.axis {
            Axis::X => {
                if grip_rect_in_window.right() < pos_in_window.x {
                    1
                } else {
                    -1
                }
            }
            Axis::Y => {
                if grip_rect_in_window.bottom() < pos_in_window.y {
                    1
                } else {
                    -1
                }
            }
        };

        Ok(())
    }

    fn pager_mouse_move(&mut self, pos_in_window: Point) -> Result<()> {
        self.pager_mouse_pos_in_window = pos_in_window;
        Ok(())
    }

    fn pager_triggered(&mut self) -> Result<()> {
        let Some(grip_rect_in_window) = self.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .children[INDEX_GRIP_IN_PAGER]
            .widget
            .common_mut()
            .rect_in_window
        else {
            return Ok(());
        };

        if self.pager_direction > 0 {
            let condition = match self.axis {
                Axis::X => grip_rect_in_window.right() < self.pager_mouse_pos_in_window.x,
                Axis::Y => grip_rect_in_window.bottom() < self.pager_mouse_pos_in_window.y,
            };
            if condition {
                self.page_forward();
            }
        } else {
            let condition = match self.axis {
                Axis::X => grip_rect_in_window.left() > self.pager_mouse_pos_in_window.x,
                Axis::Y => grip_rect_in_window.top() > self.pager_mouse_pos_in_window.y,
            };
            if condition {
                self.page_back();
            }
        }
        Ok(())
    }

    fn page_step(&self) -> i32 {
        let Some(size) = self.common.size() else {
            return 1;
        };
        match self.axis {
            Axis::X => size.x,
            Axis::Y => size.y,
        }
    }

    pub fn page_forward(&mut self) {
        self.set_value(min(
            *self.value_range.end(),
            self.current_value + self.page_step(),
        ));
    }

    pub fn page_back(&mut self) {
        self.set_value(max(
            *self.value_range.start(),
            self.current_value - self.page_step(),
        ));
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
        self.update_grip_size().or_report_err();
        if self.current_value < *self.value_range.start()
            || self.current_value > *self.value_range.end()
        {
            self.set_value(
                self.current_value
                    .clamp(*self.value_range.start(), *self.value_range.end()),
            );
        } else {
            self.update_grip_pos();
        }
        self.update_decrease_increase();
    }

    pub fn value_range(&self) -> RangeInclusive<i32> {
        self.value_range.clone()
    }

    pub fn set_step(&mut self, step: i32) {
        self.step = step;
    }

    pub fn set_value(&mut self, mut value: i32) -> &mut Self {
        if value < *self.value_range.start() || value > *self.value_range.end() {
            warn!("scroll bar value out of bounds");
            value = value.clamp(*self.value_range.start(), *self.value_range.end());
        }
        if self.current_value == value {
            return self;
        }
        self.current_value = value;
        if let Some(value_changed) = &self.value_changed {
            value_changed.invoke(self.current_value);
        }
        self.update_grip_pos();
        self.update_decrease_increase();
        self
    }

    fn update_decrease_increase(&mut self) {
        let decrease = self.common.children[INDEX_DECREASE]
            .widget
            .downcast_mut::<Button>()
            .unwrap();
        decrease.set_enabled(self.current_value > *self.value_range.start());

        let increase = self.common.children[INDEX_INCREASE]
            .widget
            .downcast_mut::<Button>()
            .unwrap();
        increase.set_enabled(self.current_value < *self.value_range.end());
    }

    fn update_grip_pos(&mut self) {
        //println!("update_grip_pos start");
        self.current_grip_pos = self.value_to_slider_pos();
        let shift = match self.axis {
            Axis::X => Point::new(self.current_grip_pos, 0),
            Axis::Y => Point::new(0, self.current_grip_pos),
        };
        let can_scroll = self.value_range.start() != self.value_range.end();
        let rect = if can_scroll {
            Some(Rect::from_pos_size(shift, self.grip_size))
        } else {
            None
        };
        self.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .set_child_rect(INDEX_GRIP_IN_PAGER, rect)
            .unwrap();

        let pager_button = self.common.children[INDEX_PAGER]
            .widget
            .downcast_mut::<Pager>()
            .unwrap()
            .common
            .children[INDEX_BUTTON_IN_PAGER]
            .widget
            .downcast_mut::<Button>()
            .unwrap();
        pager_button.set_enabled(can_scroll);
        //println!("update_grip_pos end");
    }

    pub fn value(&self) -> i32 {
        self.current_value
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

    fn update_grip_size(&mut self) -> Result<()> {
        let options = self.common.grid_options();
        let Some(size) = self.common.size() else {
            return Ok(());
        };
        let rects = grid::layout(&mut self.common.children, &options, size)?;
        self.common.set_child_rects(&rects)?;
        let pager_rect = rects.get(&INDEX_PAGER).unwrap();
        let grip_size_hint_x = self.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .children[INDEX_GRIP_IN_PAGER]
            .widget
            .size_hint_x(SizeHintMode::Preferred);
        let grip_size_hint_y = self.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .children[INDEX_GRIP_IN_PAGER]
            .widget
            .size_hint_y(grip_size_hint_x, SizeHintMode::Preferred);

        let (size_along_axis, grip_min_size_along_axis, pager_size_along_axis) = match self.axis {
            Axis::X => (size.x, grip_size_hint_x, pager_rect.size.x),
            Axis::Y => (size.y, grip_size_hint_y, pager_rect.size.y),
        };
        // For a scroll bar that's a part of a scroll area, we assume the following:
        // min value is 0; max value is the max displacement of the content in pixels;
        // size_along_axis == viewport_size;
        // max_value == content_size - viewport_size;
        // visible_ratio = viewport_size / content_size;
        // visible_ratio = viewport_size / (max_value + viewport_size);
        // we may consider adding a page_step property instead of always using size_along_axis.
        let size_plus_range = self.value_range.end() - self.value_range.start() + size_along_axis;
        let visible_ratio = if size_plus_range == 0 {
            0.0
        } else {
            (size_along_axis as f32) / (size_plus_range as f32)
        };
        let grip_len = max(
            grip_min_size_along_axis,
            (pager_size_along_axis as f32 * visible_ratio).round() as i32,
        );

        match self.axis {
            Axis::X => {
                self.grip_size = Size::new(grip_len, pager_rect.size.y);
                self.max_slider_pos = pager_rect.size.x - self.grip_size.x;
            }
            Axis::Y => {
                self.grip_size = Size::new(pager_rect.size.x, grip_len);
                self.max_slider_pos = pager_rect.size.y - self.grip_size.y;
            }
        }
        Ok(())
    }
}

impl Widget for ScrollBar {
    impl_widget_common!();

    fn new(mut common: WidgetCommonTyped<Self>) -> Self {
        let border_collapse = common.style().0.scroll_bar.border_collapse.get();
        let mut grid_options = GridOptions::ZERO;
        grid_options.x.border_collapse = border_collapse;
        grid_options.y.border_collapse = border_collapse;
        // TODO: update when style changes
        common.set_grid_options(Some(grid_options));
        // TODO: localized name

        common
            .add_child::<Button>(LayoutItemOptions::from_pos_in_grid(0, 0))
            .set_text(names::SCROLL_LEFT)
            .set_accessible(false)
            .set_focusable(false)
            .add_class("scroll_left")
            .set_text_visible(false)
            .set_auto_repeat(true)
            .set_trigger_on_press(true);

        let axis = Axis::X;
        let pager = common.add_child::<Pager>(LayoutItemOptions::from_pos_in_grid(1, 0));
        pager.set_axis(axis);
        pager.common.set_grid_options(Some(GridOptions {
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
        }));
        let mut pager_options = LayoutItemOptions::from_pos_in_grid(0, 0);
        pager_options.x.is_fixed = Some(false);
        pager_options.y.is_fixed = Some(false);
        pager
            .common
            .add_child::<Button>(pager_options)
            .set_accessible(false)
            .set_focusable(false)
            .add_class("scroll_pager")
            .set_text(names::SCROLL_PAGER)
            .set_text_visible(false)
            .set_auto_repeat(true)
            .set_trigger_on_press(true);
        pager
            .common
            .add_child::<Button>(LayoutItemOptions::default())
            .set_text(names::SCROLL_GRIP)
            .set_accessible(false)
            .set_focusable(false)
            .add_class("scroll_grip_x")
            .set_text_visible(false)
            .set_mouse_leave_sensitive(false);

        common
            .add_child::<Button>(LayoutItemOptions::from_pos_in_grid(2, 0))
            .set_text(names::SCROLL_RIGHT)
            .set_accessible(false)
            .set_focusable(false)
            .add_class("scroll_right")
            .set_text_visible(false)
            .set_auto_repeat(true)
            .set_trigger_on_press(true);
        let mut this = Self {
            common: common.into(),
            axis,
            current_grip_pos: 0,
            max_slider_pos: 0,
            grip_size: Size::default(),
            value_range: 0..=100,
            current_value: 0,
            step: 5,
            slider_grab_pos: None,
            value_changed: None,
            pager_direction: 0,
            pager_mouse_pos_in_window: Point::default(),
        };

        let slider_pressed = this.callback(Self::slider_pressed);
        let slider_moved = this.callback(Self::slider_moved);
        this.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .children[INDEX_GRIP_IN_PAGER]
            .widget
            .common_mut()
            .event_filter = Some(Box::new(move |event| {
            match event {
                Event::MouseInput(e) => {
                    if e.button == MouseButton::Left {
                        slider_pressed.invoke((e.pos_in_window, e.state));
                    }
                }
                Event::MouseMove(e) => slider_moved.invoke(e.pos_in_window),
                _ => {}
            }
            Ok(false)
        }));

        let decrease_callback = this.callback(|this, _| {
            this.decrease();
            Ok(())
        });
        this.common.children[INDEX_DECREASE]
            .widget
            .downcast_mut::<Button>()
            .unwrap()
            .on_triggered(decrease_callback);

        let increase_callback = this.callback(|this, _| {
            this.increase();
            Ok(())
        });
        this.common.children[INDEX_INCREASE]
            .widget
            .downcast_mut::<Button>()
            .unwrap()
            .on_triggered(increase_callback);

        let pager_triggered_callback = this.callback(|this, _| this.pager_triggered());
        let pager_pressed = this.callback(Self::pager_pressed);
        let pager_mouse_moved = this.callback(Self::pager_mouse_move);
        let pager_button = this.common.children[INDEX_PAGER]
            .widget
            .downcast_mut::<Pager>()
            .unwrap()
            .common
            .children[INDEX_BUTTON_IN_PAGER]
            .widget
            .downcast_mut::<Button>()
            .unwrap();
        pager_button.on_triggered(pager_triggered_callback);
        pager_button.common_mut().event_filter = Some(Box::new(move |event| {
            match event {
                Event::MouseInput(e) => {
                    if e.button == MouseButton::Left && e.state == ElementState::Pressed {
                        pager_pressed.invoke(e.pos_in_window);
                    }
                }
                Event::MouseMove(e) => pager_mouse_moved.invoke(e.pos_in_window),
                _ => {}
            }
            Ok(false)
        }));

        this.update_decrease_increase();
        this
    }

    fn handle_layout(&mut self, _event: LayoutEvent) -> Result<()> {
        self.update_grip_size()?;
        self.update_grip_pos();
        Ok(())
    }

    fn handle_mouse_scroll(&mut self, event: MouseScrollEvent) -> Result<bool> {
        let delta = event.unified_delta(&self.common);
        let max_delta = if NotNan::new(delta.x.abs())? > NotNan::new(delta.y.abs())? {
            delta.x
        } else {
            delta.y
        };
        let new_value = self.value() - max_delta.round() as i32;
        self.set_value(new_value.clamp(*self.value_range.start(), *self.value_range.end()));
        Ok(true)
    }

    fn handle_keyboard_input(&mut self, event: KeyboardInputEvent) -> Result<bool> {
        if !event.info.state.is_pressed() {
            return Ok(false);
        }
        if let Key::Named(key) = event.info.logical_key {
            match key {
                NamedKey::ArrowDown => {
                    self.increase();
                    Ok(true)
                }
                NamedKey::ArrowLeft => {
                    self.decrease();
                    Ok(true)
                }
                NamedKey::ArrowRight => {
                    self.increase();
                    Ok(true)
                }
                NamedKey::ArrowUp => {
                    self.decrease();
                    Ok(true)
                }
                NamedKey::End => {
                    self.set_value(*self.value_range().end());
                    Ok(true)
                }
                NamedKey::Home => {
                    self.set_value(*self.value_range().start());
                    Ok(true)
                }
                NamedKey::PageDown => {
                    self.page_forward();
                    Ok(true)
                }
                NamedKey::PageUp => {
                    self.page_back();
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    fn handle_focus_in(&mut self, _event: FocusInEvent) -> Result<()> {
        self.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .children[INDEX_GRIP_IN_PAGER]
            .widget
            .add_class("scroll_bar_focused");
        Ok(())
    }

    fn handle_focus_out(&mut self, _event: FocusOutEvent) -> Result<()> {
        self.common.children[INDEX_PAGER]
            .widget
            .common_mut()
            .children[INDEX_GRIP_IN_PAGER]
            .widget
            .remove_class("scroll_bar_focused");
        Ok(())
    }
}

struct Pager {
    common: WidgetCommon,
    axis: Axis,
}

impl Pager {
    pub fn set_axis(&mut self, axis: Axis) {
        self.axis = axis;
        self.common.size_hint_changed();
    }
}

const PAGER_SIZE_HINT_MULTIPLIER: i32 = 2;

impl Widget for Pager {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
            axis: Axis::X,
        }
    }

    fn recalculate_size_hint_x(&mut self, mode: SizeHintMode) -> Result<i32> {
        let grip_hint = self.common.children[INDEX_GRIP_IN_PAGER]
            .widget
            .size_hint_x(mode);
        match self.axis {
            Axis::X => Ok(grip_hint * PAGER_SIZE_HINT_MULTIPLIER),
            Axis::Y => Ok(grip_hint),
        }
    }
    fn recalculate_size_hint_y(&mut self, size_x: i32, mode: SizeHintMode) -> Result<i32> {
        let grip_hint = self.common.children[INDEX_GRIP_IN_PAGER]
            .widget
            .size_hint_y(size_x, mode);
        match self.axis {
            Axis::X => Ok(grip_hint),
            Axis::Y => Ok(grip_hint * PAGER_SIZE_HINT_MULTIPLIER),
        }
    }
    fn recalculate_size_x_fixed(&mut self) -> bool {
        self.axis == Axis::Y
    }
    fn recalculate_size_y_fixed(&mut self) -> bool {
        self.axis == Axis::X
    }
}
