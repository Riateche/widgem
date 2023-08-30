use cosmic_text::{BorrowedWithFontSystem, Buffer, Editor, SwashCache};
use tiny_skia::{Color, Paint, Pixmap, PixmapPaint, PixmapRef, Transform};

use crate::types::{Point, Rect, Size};

pub struct DrawContext<'a> {
    pub pixmap: &'a mut Pixmap,
    pub rect: Rect,
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
    buffer: &mut impl DrawableTextBuffer,
    size: Size,
    color: Color,
    swash_cache: &mut SwashCache,
) -> Pixmap {
    // TODO: empty size check
    // TODO: error propagation?
    let mut pixmap = Pixmap::new(size.x as u32, size.y as u32).expect("failed to create pixmap");
    buffer.draw(swash_cache, convert_color(color), |x, y, w, h, c| {
        let color = Color::from_rgba8(c.r(), c.g(), c.b(), c.a());
        let paint = Paint {
            shader: tiny_skia::Shader::SolidColor(color),
            ..Paint::default()
        };
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(x as f32, y as f32, w as f32, h as f32).unwrap(),
            &paint,
            Transform::default(),
            None,
        );
    });
    pixmap
}

pub trait DrawableTextBuffer {
    fn draw<F>(&mut self, cache: &mut SwashCache, color: cosmic_text::Color, f: F)
    where
        F: FnMut(i32, i32, u32, u32, cosmic_text::Color);
}

impl DrawableTextBuffer for BorrowedWithFontSystem<'_, Buffer> {
    fn draw<F>(&mut self, cache: &mut SwashCache, color: cosmic_text::Color, f: F)
    where
        F: FnMut(i32, i32, u32, u32, cosmic_text::Color),
    {
        self.draw(cache, color, f)
    }
}

impl DrawableTextBuffer for BorrowedWithFontSystem<'_, Editor> {
    fn draw<F>(&mut self, cache: &mut SwashCache, color: cosmic_text::Color, f: F)
    where
        F: FnMut(i32, i32, u32, u32, cosmic_text::Color),
    {
        self.draw(cache, color, f)
    }
}

fn convert_color(color: Color) -> cosmic_text::Color {
    let c = color.to_color_u8();
    cosmic_text::Color::rgba(c.red(), c.green(), c.blue(), c.alpha())
}

// fn convert_color_back(c: cosmic_text::Color) -> Color {
//     Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
// }
