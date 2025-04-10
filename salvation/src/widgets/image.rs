use {
    super::{Widget, WidgetCommon, WidgetCommonTyped},
    crate::{draw::DrawEvent, impl_widget_common, layout::SizeHintMode, types::Point},
    anyhow::Result,
    png::DecodingError,
    salvation_macros::impl_with,
    std::{path::Path, rc::Rc},
    tiny_skia::Pixmap,
    usvg::Transform,
};

pub struct Image {
    pixmap: Option<Rc<Pixmap>>,
    // TODO: finite f32
    scale: Option<f32>,
    common: WidgetCommon,
    is_prescaled: bool,
}

#[impl_with]
impl Image {
    pub fn set_prescaled(&mut self, value: bool) {
        self.is_prescaled = value;
        self.common.size_hint_changed();
        self.common.update();
    }

    pub fn is_prescaled(&self) -> bool {
        self.is_prescaled
    }

    pub fn set_pixmap(&mut self, pixmap: Option<Rc<Pixmap>>) {
        if self.pixmap.as_ref().map(Rc::as_ptr) == pixmap.as_ref().map(Rc::as_ptr) {
            return;
        }
        self.pixmap = pixmap;
        self.common.size_hint_changed();
        self.common.update();
    }

    pub fn load_png<P: AsRef<Path>>(&mut self, path: P) -> Result<(), DecodingError> {
        self.set_pixmap(Some(Rc::new(Pixmap::load_png(path)?)));
        Ok(())
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
        let extra_scale = if self.is_prescaled {
            1.0
        } else {
            self.common.style().0.image.scale
        };
        self.scale.unwrap_or(1.0) * extra_scale
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

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
            pixmap: None,
            is_prescaled: false,
            scale: None,
        }
    }

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let scale = self.total_scale();
        if let Some(pixmap) = &self.pixmap {
            event.draw_pixmap(
                Point::default(),
                (**pixmap).as_ref(),
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

    fn handle_layout(&mut self, _event: crate::event::LayoutEvent) -> Result<()> {
        Ok(())
    }
}
