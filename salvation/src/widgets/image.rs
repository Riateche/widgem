use std::path::Path;

use anyhow::Result;
use png::DecodingError;
use salvation_macros::impl_with;
use tiny_skia::Pixmap;
use usvg::Transform;

use crate::{draw::DrawEvent, impl_widget_common, layout::SizeHintMode, types::Point};

use super::{Widget, WidgetCommon};

pub struct Image {
    pixmap: Option<Pixmap>,
    // TODO: finite f32
    scale: Option<f32>,
    common: WidgetCommon,
}

#[impl_with]
impl Image {
    pub fn load_png<P: AsRef<Path>>(path: P) -> Result<Self, DecodingError> {
        Ok(Self::new(Some(Pixmap::load_png(path)?)))
    }

    pub fn new(pixmap: Option<Pixmap>) -> Self {
        Self {
            pixmap,
            common: WidgetCommon::new::<Self>().into(),
            scale: None,
        }
    }

    pub fn set_pixmap(&mut self, pixmap: Option<Pixmap>) {
        self.pixmap = pixmap;
        self.common.size_hint_changed();
        self.common.update();
    }

    pub fn set_scale(&mut self, scale: Option<f32>) {
        if self.scale == scale {
            return;
        }
        self.scale = scale;
        self.common.size_hint_changed();
        self.common.update();
    }

    fn total_scale(&self) -> f32 {
        self.scale.unwrap_or(1.0) * self.common.style().0.image.scale
    }

    pub fn map_widget_pos_to_content_pos(&self, pos: Point) -> Point {
        let scale = self.total_scale();
        Point::new(
            ((pos.x as f32) / scale).round() as i32,
            ((pos.y as f32) / scale).round() as i32,
        )
    }
}

impl Widget for Image {
    impl_widget_common!();

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let scale = self.total_scale();
        if let Some(pixmap) = &self.pixmap {
            event.draw_pixmap(
                Point::default(),
                pixmap.as_ref(),
                Transform::from_scale(scale, scale),
            );
        }
        Ok(())
    }

    fn recalculate_size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        let scale = self.total_scale();
        Ok((self.pixmap.as_ref().map_or(0.0, |p| p.width() as f32) * scale).ceil() as i32)
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        let scale = self.total_scale();
        Ok((self.pixmap.as_ref().map_or(0.0, |p| p.height() as f32) * scale).ceil() as i32)
    }
}
