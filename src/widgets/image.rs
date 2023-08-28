use std::path::Path;

use png::DecodingError;
use tiny_skia::Pixmap;

use crate::{draw::DrawContext, types::Point};

use super::{Widget, WidgetCommon};

pub struct Image {
    pixmap: Option<Pixmap>,
    common: WidgetCommon,
}

impl Image {
    pub fn load_png<P: AsRef<Path>>(path: P) -> Result<Self, DecodingError> {
        Ok(Self {
            pixmap: Some(Pixmap::load_png(path)?),
            common: WidgetCommon::new(),
        })
    }

    pub fn new(pixmap: Pixmap) -> Self {
        Self {
            pixmap: Some(pixmap),
            common: WidgetCommon::new(),
        }
    }
}

impl Widget for Image {
    fn draw(&mut self, ctx: &mut DrawContext<'_>) {
        if let Some(pixmap) = &self.pixmap {
            ctx.draw_pixmap(Point::default(), pixmap.as_ref());
        }
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }
}
