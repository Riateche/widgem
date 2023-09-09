use std::{cell::RefCell, rc::Rc};

use cosmic_text::{BorrowedWithFontSystem, Buffer, Editor, SwashCache};
use tiny_skia::{
    BlendMode, Color, FilterQuality, Paint, PathBuilder, Pattern, Pixmap, PixmapPaint, PixmapRef,
    SpreadMode, Stroke, Transform,
};

use crate::types::{Point, Rect, Size};

pub struct DrawEvent {
    pub pixmap: Rc<RefCell<Pixmap>>,
    pub rect: Rect,
}

impl DrawEvent {
    pub fn draw_pixmap(&self, pos: Point, pixmap: PixmapRef<'_>) {
        self.pixmap.borrow_mut().draw_pixmap(
            pos.x + self.rect.top_left.x,
            pos.y + self.rect.top_left.y,
            pixmap,
            &PixmapPaint::default(),
            Transform::default(),
            // TODO: mask?
            None,
        )
    }

    pub fn draw_subpixmap(&self, target_rect: Rect, pixmap: PixmapRef<'_>, pixmap_offset: Point) {
        let target_rect = target_rect.translate(self.rect.top_left);
        let translation = target_rect.top_left - pixmap_offset;
        let patt_transform = Transform::from_translate(translation.x as f32, translation.y as f32);
        let paint = Paint {
            shader: Pattern::new(
                pixmap,
                SpreadMode::Pad, // Pad, otherwise we will get weird borders overlap.
                FilterQuality::Nearest,
                1.0,
                patt_transform,
            ),
            blend_mode: BlendMode::default(),
            anti_alias: false,        // Skia doesn't use it too.
            force_hq_pipeline: false, // Pattern will use hq anyway.
        };

        self.pixmap.borrow_mut().fill_rect(
            tiny_skia::Rect::from_xywh(
                target_rect.top_left.x as f32,
                target_rect.top_left.y as f32,
                target_rect.size.x as f32,
                target_rect.size.y as f32,
            )
            .unwrap(),
            &paint,
            Transform::default(),
            None,
        );
    }

    pub fn draw_rounded_rect(&self, rect: Rect, radius: f32, color: Color, width: f32) {
        if radius > (rect.size.x as f32 / 2.0) || radius > (rect.size.y as f32 / 2.0) {
            //TODO do something here, log some error
            println!("radius is bigger than fits in rectangle");
            return;
        }
        let top_left_point = self.rect.top_left + rect.top_left;
        let top_left = (
            top_left_point.x as f32 + width / 2.0,
            top_left_point.y as f32 + width / 2.0,
        );
        let size = (rect.size.x as f32 - width, rect.size.y as f32 - width);
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(top_left.0 + radius, top_left.1);
        path_builder.line_to(top_left.0 + size.0 - radius, top_left.1);
        Self::rounded_line_in_square_corner(
            &mut path_builder,
            top_left.0 + size.0,
            top_left.1,
            top_left.0 + size.0,
            top_left.1 + radius,
        );
        path_builder.line_to(top_left.0 + size.0, top_left.1 + size.1 - radius);
        Self::rounded_line_in_square_corner(
            &mut path_builder,
            top_left.0 + size.0,
            top_left.1 + size.1,
            top_left.0 + size.0 - radius,
            top_left.1 + size.1,
        );
        path_builder.line_to(top_left.0 + radius, top_left.1 + size.1);
        Self::rounded_line_in_square_corner(
            &mut path_builder,
            top_left.0,
            top_left.1 + size.1,
            top_left.0,
            top_left.1 + size.1 - radius,
        );
        path_builder.line_to(top_left.0, top_left.1 + radius);
        Self::rounded_line_in_square_corner(
            &mut path_builder,
            top_left.0,
            top_left.1,
            top_left.0 + radius,
            top_left.1,
        );
        let path = path_builder.finish().unwrap();
        self.pixmap.borrow_mut().stroke_path(
            &path,
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            &Stroke {
                width,
                ..Stroke::default()
            },
            Transform::default(),
            // TODO: mask?
            None,
        );
    }

    fn rounded_line_in_square_corner(
        path_builder: &mut PathBuilder,
        corner_x: f32,
        corner_y: f32,
        x: f32,
        y: f32,
    ) {
        path_builder.quad_to(
            corner_x + (x - corner_x) / 4.0,
            corner_y + (y - corner_y) / 4.0,
            x,
            y,
        );
    }

    // TODO: add at least width
    pub fn stroke_rect(&self, rect: Rect, color: Color) {
        let top_left = self.rect.top_left + rect.top_left;
        let path = PathBuilder::from_rect(
            tiny_skia::Rect::from_xywh(
                top_left.x as f32 + 0.5,
                top_left.y as f32 + 0.5,
                rect.size.x as f32 - 1.0,
                rect.size.y as f32 - 1.0,
            )
            .unwrap(),
        );
        self.pixmap.borrow_mut().stroke_path(
            &path,
            &Paint {
                shader: tiny_skia::Shader::SolidColor(color),
                ..Paint::default()
            },
            &Stroke::default(),
            Transform::default(),
            // TODO: mask?
            None,
        );
    }

    pub fn fill_rect(&self, rect: Rect, color: Color) {
        let top_left = self.rect.top_left + rect.top_left;
        self.pixmap.borrow_mut().fill_rect(
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
    pub unfocused_input_border: Color,
    pub focused_input_border: Color,
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

pub fn convert_color(color: Color) -> cosmic_text::Color {
    let c = color.to_color_u8();
    cosmic_text::Color::rgba(c.red(), c.green(), c.blue(), c.alpha())
}

// fn convert_color_back(c: cosmic_text::Color) -> Color {
//     Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
// }
