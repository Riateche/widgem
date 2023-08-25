use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Color, Pixmap, PixmapPaint, PixmapRef, Transform};

use crate::types::{Point, Rect};

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
}

pub struct Palette {
    pub foreground: Color,
    pub background: Color,
}
