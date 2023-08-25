use cosmic_text::{BorrowedWithFontSystem, Buffer, FontSystem, SwashCache};
use tiny_skia::{Color, ColorU8, Paint, Pixmap, PixmapPaint, PixmapRef, Transform};

use crate::types::{Point, Rect, Size};

pub struct DrawContext<'a> {
    pub pixmap: &'a mut Pixmap,
    pub rect: Rect,
    pub font_system: &'a mut FontSystem,
    pub font_metrics: cosmic_text::Metrics,
    pub swash_cache: &'a mut SwashCache,
    pub palette: &'a mut Palette,
}

impl DrawContext<'_> {
    pub fn draw_pixmap(&mut self, pos: Point, pixmap: PixmapRef<'_>) {
        self.pixmap.draw_pixmap(
            pos.x + self.rect.top_left.x,
            pos.y + self.rect.top_left.y,
            pixmap,
            &PixmapPaint::default(),
            Transform::default(),
            // TODO: mask?
            None,
        )
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let top_left = self.rect.top_left + rect.top_left;
        self.pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(
                top_left.x as f32,
                top_left.y as f32,
                rect.size.x as f32,
                rect.size.y as f32,
            )
            .unwrap(),
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            Transform::default(),
            // TODO: mask?
            None,
        );
    }
}

pub struct Palette {
    pub foreground: Color,
    pub background: Color,
}

const MEASURE_MAX_SIZE: f32 = 10_000.;

// TODO: move to font metrics wrapper?
pub fn unrestricted_text_size(buffer: &mut BorrowedWithFontSystem<'_, Buffer>) -> Size {
    buffer.set_size(MEASURE_MAX_SIZE, MEASURE_MAX_SIZE);
    buffer.shape_until_scroll();
    let height = (buffer.lines.len() as f32 * buffer.metrics().line_height).ceil() as i32;
    let width = buffer
        .layout_runs()
        .map(|run| run.line_w.ceil() as i32)
        .max()
        .unwrap_or(0);

    Size {
        x: width,
        y: height,
    }
}

pub fn draw_text(
    buffer: &mut BorrowedWithFontSystem<'_, Buffer>,
    size: Size,
    color: Color,
    swash_cache: &mut SwashCache,
) -> Pixmap {
    // TODO: empty size check
    // TODO: error propagation?
    let mut pixmap = Pixmap::new(size.x as u32, size.y as u32).expect("failed to create pixmap");
    let pixels = pixmap.pixels_mut();
    buffer.draw(swash_cache, convert_color(color), |x, y, w, h, c| {
        for dx in 0..w {
            for dy in 0..h {
                let x = x as usize + dx as usize;
                let y = y as usize + dy as usize;
                pixels[y * size.x as usize + x] =
                    ColorU8::from_rgba(c.r(), c.g(), c.b(), c.a()).premultiply();
            }
        }
    });
    pixmap
}

fn convert_color(color: Color) -> cosmic_text::Color {
    let c = color.to_color_u8();
    cosmic_text::Color::rgba(c.red(), c.green(), c.blue(), c.alpha())
}

// fn convert_color_back(c: cosmic_text::Color) -> Color {
//     Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
// }
