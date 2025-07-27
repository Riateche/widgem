use std::{path::Path, rc::Rc};

use anyhow::Context as _;

use crate::types::{PhysicalPixels, PpxSuffix, Size};

#[derive(Debug, Clone)]
pub struct Pixmap(Rc<tiny_skia::Pixmap>);

impl PartialEq for Pixmap {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Pixmap {}

impl Pixmap {
    pub fn load_png<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        tiny_skia::Pixmap::load_png(path.as_ref())
            .with_context(|| format!("failed to load file as png: {:?}", path.as_ref().display()))
            .map(Rc::new)
            .map(Self)
    }

    pub(crate) fn as_tiny_skia_ref(&self) -> tiny_skia::PixmapRef<'_> {
        (*self.0).as_ref()
    }

    pub fn size(&self) -> Size {
        Size::new(
            (self.0.width() as i32).ppx(),
            (self.0.height() as i32).ppx(),
        )
    }

    pub fn size_x(&self) -> PhysicalPixels {
        (self.0.width() as i32).ppx()
    }

    pub fn size_y(&self) -> PhysicalPixels {
        (self.0.height() as i32).ppx()
    }
}

impl From<tiny_skia::Pixmap> for Pixmap {
    fn from(value: tiny_skia::Pixmap) -> Self {
        Self(Rc::new(value))
    }
}

impl From<Rc<tiny_skia::Pixmap>> for Pixmap {
    fn from(value: Rc<tiny_skia::Pixmap>) -> Self {
        Self(value)
    }
}
