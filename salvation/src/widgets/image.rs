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
    common: WidgetCommon,
}

#[impl_with]
impl Image {
    pub fn load_png<P: AsRef<Path>>(path: P) -> Result<Self, DecodingError> {
        Ok(Self {
            pixmap: Some(Pixmap::load_png(path)?),
            common: WidgetCommon::new(),
        })
    }

    pub fn new(pixmap: Option<Pixmap>) -> Self {
        Self {
            pixmap,
            common: WidgetCommon::new(),
        }
    }

    pub fn set_pixmap(&mut self, pixmap: Option<Pixmap>) {
        self.pixmap = pixmap;
        self.common.size_hint_changed();
        self.common.update();
    }
}

impl Widget for Image {
    impl_widget_common!();

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        if let Some(pixmap) = &self.pixmap {
            event.draw_pixmap(
                Point::default(),
                pixmap.as_ref(),
                Transform::default(), //from_scale(self.common.style().scale, self.common.style().scale),
            );
        }
        Ok(())
    }

    fn recalculate_size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        Ok(
            (self.pixmap.as_ref().map_or(0.0, |p| p.width() as f32) * self.common.style().scale)
                .ceil() as i32,
        )
    }

    fn recalculate_size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        Ok(
            (self.pixmap.as_ref().map_or(0.0, |p| p.height() as f32) * self.common.style().scale)
                .ceil() as i32,
        )
    }
}
