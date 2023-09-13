use std::{cmp::max, fmt::Display};

use cosmic_text::{Attrs, Buffer, Shaping};
use tiny_skia::{Color, GradientStop, LinearGradient, Pixmap, SpreadMode, Transform};

use crate::{
    callback::Callback,
    draw::{draw_text, unrestricted_text_size, DrawEvent},
    event::MouseInputEvent,
    system::with_system,
    types::{Point, Rect, Size},
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
    common: WidgetCommon,
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
            common,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text = text.to_string();
        self.redraw_text = true;
    }

    pub fn on_clicked(&mut self, callback: Callback<String>) {
        self.on_clicked = Some(callback);
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
        let gradient = LinearGradient::new(
            start,
            end,
            vec![
                GradientStop::new(0.0, Color::from_rgba8(0, 0, 0, 255)),
                GradientStop::new(1.0, Color::from_rgba8(255, 255, 255, 255)),
            ],
            SpreadMode::Pad,
            Transform::default(),
        )
        .expect("failed to create gradient");
        event.stroke_and_fill_rounded_rect(
            Rect {
                top_left: Point::default(),
                size: event.rect.size,
            },
            2.0,
            1.0,
            gradient,
            Color::from_rgba8(220, 220, 220, 255),
        );

        with_system(|system| {
            let mut buffer = self
                .buffer
                .get_or_insert_with(|| Buffer::new(&mut system.font_system, system.font_metrics))
                .borrow_with(&mut system.font_system);

            if self.redraw_text {
                buffer.set_text(&self.text, Attrs::new(), Shaping::Advanced);
                self.unrestricted_text_size = unrestricted_text_size(&mut buffer);
                let pixmap = draw_text(
                    &mut buffer,
                    self.unrestricted_text_size,
                    system.palette.foreground,
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

    fn on_mouse_input(&mut self, _event: MouseInputEvent) -> bool {
        if let Some(on_clicked) = &self.on_clicked {
            on_clicked.invoke(self.text.clone());
        }
        true
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}
