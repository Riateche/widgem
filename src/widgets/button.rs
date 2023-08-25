use std::{cmp::max, fmt::Display};

use cosmic_text::{Attrs, Buffer, Shaping};
use tiny_skia::{Color, Pixmap};

use crate::{
    draw::{draw_text, unrestricted_text_size, DrawContext},
    event::MouseInputEvent,
    types::{Point, Rect, Size},
    Widget,
};

pub struct Button {
    text: String,
    buffer: Option<Buffer>,
    text_pixmap: Option<Pixmap>,
    unrestricted_text_size: Size,
    redraw_text: bool,
}

impl Button {
    pub fn new(text: impl Display) -> Self {
        Self {
            text: text.to_string(),
            buffer: None,
            text_pixmap: None,
            unrestricted_text_size: Size::default(),
            redraw_text: true,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text = text.to_string();
        self.redraw_text = true;
    }
}

impl Widget for Button {
    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        ctx.fill_rect(
            Rect {
                top_left: Point::default(),
                size: ctx.rect.size,
            },
            Color::from_rgba8(180, 255, 180, 255),
        );
        ctx.fill_rect(
            Rect {
                top_left: Point { x: 3, y: 3 },
                size: Size {
                    x: ctx.rect.size.x - 6,
                    y: ctx.rect.size.y - 6,
                },
            },
            Color::from_rgba8(220, 220, 220, 255),
        );

        let mut buffer = self
            .buffer
            .get_or_insert_with(|| Buffer::new(ctx.font_system, ctx.font_metrics))
            .borrow_with(ctx.font_system);

        if self.redraw_text {
            buffer.set_text(&self.text, Attrs::new(), Shaping::Advanced);
            self.unrestricted_text_size = unrestricted_text_size(&mut buffer);
            let pixmap = draw_text(
                &mut buffer,
                self.unrestricted_text_size,
                ctx.palette.foreground,
                ctx.swash_cache,
            );
            self.text_pixmap = Some(pixmap);
            self.redraw_text = false;
        }

        if let Some(pixmap) = &self.text_pixmap {
            let padding = Point {
                x: max(0, ctx.rect.size.x - pixmap.width() as i32) / 2,
                y: max(0, ctx.rect.size.y - pixmap.height() as i32) / 2,
            };
            ctx.draw_pixmap(padding, pixmap.as_ref());
        }
    }

    fn mouse_input(&mut self, _event: &mut MouseInputEvent<'_>) {
        println!("button clicked {:?}", self.text);
    }
}
