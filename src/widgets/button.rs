use std::{cmp::max, fmt::Display};

use accesskit::{Action, DefaultActionVerb, NodeBuilder, Role};
use cosmic_text::{Attrs, Buffer, Shaping};
use tiny_skia::{Color, GradientStop, LinearGradient, Pixmap, SpreadMode, Transform};
use winit::event::MouseButton;

use crate::{
    callback::Callback,
    draw::{draw_text, unrestricted_text_size, DrawEvent},
    event::CursorMovedEvent,
    event::{AccessibleEvent, FocusReason, MouseInputEvent},
    layout::SizeHint,
    system::{send_window_request, with_system},
    types::{Point, Rect, Size},
    window::SetFocusRequest,
};

use super::{Widget, WidgetCommon};

pub struct Button {
    text: String,
    buffer: Option<Buffer>,
    text_pixmap: Option<Pixmap>,
    unrestricted_text_size: Size,
    redraw_text: bool,
    // TODO: Option inside callback
    on_clicked: Option<Callback<String>>,
    state: ButtonState,
    enabled: bool,
    common: WidgetCommon,
}

#[derive(PartialEq)]
enum ButtonState {
    Default,
    Hover,
    Pressed,
}

impl Button {
    pub fn new(text: impl Display) -> Self {
        let mut common = WidgetCommon::new();
        common.is_focusable = true;
        Self {
            text: text.to_string(),
            buffer: None,
            text_pixmap: None,
            unrestricted_text_size: Size::default(),
            redraw_text: true,
            on_clicked: None,
            enabled: true,
            state: ButtonState::Default,
            common,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text = text.to_string();
        self.redraw_text = true;
    }

    //TODO: needs some automatic redraw?
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
        }
    }

    pub fn on_clicked(&mut self, callback: Callback<String>) {
        self.on_clicked = Some(callback);
    }

    fn click(&mut self) {
        if let Some(on_clicked) = &self.on_clicked {
            on_clicked.invoke(self.text.clone());
        }
    }
}

impl Widget for Button {
    fn on_draw(&mut self, event: DrawEvent) {
        let start = tiny_skia::Point {
            x: event.rect.top_left.x as f32,
            y: event.rect.top_left.y as f32,
        };
        let end = tiny_skia::Point {
            x: event.rect.top_left.x as f32,
            y: event.rect.top_left.y as f32 + event.rect.size.y as f32,
        };
        let gradient = if !self.enabled {
            LinearGradient::new(
                start,
                end,
                vec![
                    GradientStop::new(0.0, Color::from_rgba8(254, 254, 254, 255)),
                    GradientStop::new(1.0, Color::from_rgba8(238, 238, 238, 255)),
                ],
                SpreadMode::Pad,
                Transform::default(),
            )
        } else {
            match self.state {
                ButtonState::Default => LinearGradient::new(
                    start,
                    end,
                    vec![
                        GradientStop::new(0.0, Color::from_rgba8(254, 254, 254, 255)),
                        GradientStop::new(1.0, Color::from_rgba8(238, 238, 238, 255)),
                    ],
                    SpreadMode::Pad,
                    Transform::default(),
                ),
                ButtonState::Hover => LinearGradient::new(
                    start,
                    end,
                    vec![
                        GradientStop::new(1.0, Color::from_rgba8(254, 254, 254, 255)),
                        GradientStop::new(1.0, Color::from_rgba8(247, 247, 247, 255)),
                    ],
                    SpreadMode::Pad,
                    Transform::default(),
                ),
                ButtonState::Pressed => LinearGradient::new(
                    start,
                    end,
                    vec![GradientStop::new(
                        1.0,
                        Color::from_rgba8(219, 219, 219, 255),
                    )],
                    SpreadMode::Pad,
                    Transform::default(),
                ),
            }
        }
        .expect("failed to create gradient");
        let border_color = if self.enabled {
            if self.common.is_focused {
                Color::from_rgba8(38, 112, 158, 255)
            } else {
                Color::from_rgba8(171, 171, 171, 255)
            }
        } else {
            Color::from_rgba8(196, 196, 196, 255)
        };
        event.stroke_and_fill_rounded_rect(
            Rect {
                top_left: Point::default(),
                size: event.rect.size,
            },
            2.0,
            1.0,
            gradient,
            border_color,
        );

        with_system(|system| {
            let mut buffer = self
                .buffer
                .get_or_insert_with(|| Buffer::new(&mut system.font_system, system.font_metrics))
                .borrow_with(&mut system.font_system);

            let text_color = if self.enabled {
                system.palette.foreground
            } else {
                Color::from_rgba8(191, 191, 191, 255)
            };

            if self.redraw_text {
                buffer.set_text(&self.text, Attrs::new(), Shaping::Advanced);
                self.unrestricted_text_size = unrestricted_text_size(&mut buffer);
                let pixmap = draw_text(
                    &mut buffer,
                    self.unrestricted_text_size,
                    text_color,
                    &mut system.swash_cache,
                );
                self.text_pixmap = Some(pixmap);
                self.redraw_text = false;
            }

            if let Some(pixmap) = &self.text_pixmap {
                let padding = Point {
                    x: max(0, event.rect.size.x - pixmap.width() as i32) / 2,
                    y: max(0, event.rect.size.y - pixmap.height() as i32) / 2,
                };
                event.draw_pixmap(padding, pixmap.as_ref());
            }
        });
    }

    fn on_mouse_input(&mut self, event: MouseInputEvent) -> bool {
        // TODO: only on release, check buttons
        if event.button == MouseButton::Left {
            if event.state.is_pressed() {
                if self.enabled {
                    self.state = ButtonState::Pressed;
                    self.click();
                }
            } else if self.enabled {
                self.state = ButtonState::Hover;
            }
        }

        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");
        send_window_request(
            mount_point.address.window_id,
            SetFocusRequest {
                widget_id: self.common.id,
                reason: FocusReason::Mouse,
            },
        );
        true
    }

    // TODO: mouse out event
    fn on_cursor_moved(&mut self, _event: CursorMovedEvent) -> bool {
        self.state = ButtonState::Hover;
        false
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
    fn on_accessible(&mut self, event: AccessibleEvent) {
        let mount_point = &self
            .common
            .mount_point
            .as_ref()
            .expect("cannot handle event when unmounted");

        match event.action {
            Action::Default => self.click(),
            Action::Focus => {
                send_window_request(
                    mount_point.address.window_id,
                    SetFocusRequest {
                        widget_id: self.common.id,
                        // TODO: separate reason?
                        reason: FocusReason::Mouse,
                    },
                );
            }
            _ => {}
        }
    }

    fn accessible_node(&mut self) -> Option<accesskit::NodeBuilder> {
        let mut node = NodeBuilder::new(Role::Button);
        node.set_name(self.text.as_str());
        node.add_action(Action::Focus);
        //node.add_action(Action::Default);
        node.set_default_action_verb(DefaultActionVerb::Click);
        Some(node)
    }

    fn size_hint_x(&mut self) -> SizeHint {
        SizeHint {
            min: 150,
            preferred: 150,
            is_fixed: true,
        }
    }

    fn size_hint_y(&mut self, _size_x: i32) -> SizeHint {
        SizeHint {
            min: 30,
            preferred: 30,
            is_fixed: true,
        }
    }
}
