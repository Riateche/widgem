use std::fmt::Display;

use cosmic_text::{Attrs, Buffer, Shaping};
use tiny_skia::{Color, ColorU8, Pixmap};

use crate::{
    draw::DrawContext,
    types::{Point, Size},
    Widget,
};

pub struct Label {
    text: String,
    buffer: Option<Buffer>,
    pixmap: Option<Pixmap>,
    unrestricted_text_size: Size,
    text_updated: bool,
}

impl Label {
    pub fn new(text: impl Display) -> Self {
        Self {
            text: text.to_string(),
            buffer: None,
            pixmap: None,
            unrestricted_text_size: Size::default(),
            text_updated: true,
        }
    }

    pub fn set_text(&mut self, text: impl Display) {
        self.text = text.to_string();
        self.text_updated = true;
    }
}

impl Widget for Label {
    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        let mut buffer = self
            .buffer
            .get_or_insert_with(|| Buffer::new(ctx.font_system, ctx.font_metrics))
            .borrow_with(ctx.font_system);

        if self.text_updated {
            buffer.set_size(MEASURE_MAX_SIZE, MEASURE_MAX_SIZE);
            buffer.set_text(&self.text, Attrs::new(), Shaping::Advanced);
            buffer.shape_until_scroll();
            let height = (buffer.lines.len() as f32 * buffer.metrics().line_height).ceil() as i32;
            let width = buffer
                .layout_runs()
                .map(|run| run.line_w.ceil() as i32)
                .max()
                .unwrap_or(0);

            self.unrestricted_text_size = Size {
                x: width,
                y: height,
            };
            // TODO: error propagation?
            let mut pixmap =
                Pixmap::new(width as u32, height as u32).expect("failed to create pixmap");
            let pixels = pixmap.pixels_mut();
            buffer.draw(
                ctx.swash_cache,
                convert_color(ctx.palette.foreground),
                |x, y, w, h, c| {
                    for dx in 0..w {
                        for dy in 0..h {
                            let x = x as usize + dx as usize;
                            let y = y as usize + dy as usize;
                            pixels[y * width as usize + x] =
                                ColorU8::from_rgba(c.r(), c.g(), c.b(), c.a()).premultiply();
                        }
                    }
                    // pixmap.fill_rect(
                    //     tiny_skia::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32)
                    //         .expect("invalid coords"),
                    //     &Paint {
                    //         shader: tiny_skia::Shader::SolidColor(convert_color_back(color)),
                    //         ..Paint::default()
                    //     },
                    //     Transform::default(),
                    //     None,
                    // );
                },
            );
            self.pixmap = Some(pixmap);
            self.text_updated = false;
        }

        if let Some(pixmap) = &self.pixmap {
            ctx.draw_pixmap(Point::default(), pixmap.as_ref());
        }
    }
}

fn convert_color(color: Color) -> cosmic_text::Color {
    let c = color.to_color_u8();
    cosmic_text::Color::rgba(c.red(), c.green(), c.blue(), c.alpha())
}

// fn convert_color_back(c: cosmic_text::Color) -> Color {
//     Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
// }

const MEASURE_MAX_SIZE: f32 = 10_000.;
