use {
    super::{Widget, WidgetBaseOf},
    crate::{
        draw::DrawEvent,
        impl_widget_base,
        layout::SizeHint,
        types::{PhysicalPixels, Point, PpxSuffix},
        widgets::widget_trait::NewWidget,
        Pixmap,
    },
    anyhow::Result,
    std::path::Path,
    usvg::Transform,
    widgem_macros::impl_with,
};

pub struct Image {
    pixmap: Option<Pixmap>,
    // TODO: finite f32
    scale: Option<f32>,
    base: WidgetBaseOf<Self>,
    is_prescaled: bool,
}

#[impl_with]
impl Image {
    pub fn set_prescaled(&mut self, value: bool) {
        self.is_prescaled = value;
        self.base.size_hint_changed();
        self.base.update();
    }

    pub fn is_prescaled(&self) -> bool {
        self.is_prescaled
    }

    pub fn set_pixmap(&mut self, pixmap: Option<Pixmap>) {
        if self.pixmap == pixmap {
            return;
        }
        self.pixmap = pixmap;
        self.base.size_hint_changed();
        self.base.update();
    }

    pub fn load_png<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.set_pixmap(Some(Pixmap::load_png(path)?));
        Ok(())
    }

    pub fn set_scale(&mut self, scale: Option<f32>) -> &mut Self {
        if self.scale == scale {
            return self;
        }
        self.scale = scale;
        self.base.size_hint_changed();
        self.base.update();
        self
    }

    fn total_scale(&self) -> f32 {
        let extra_scale = if self.is_prescaled {
            1.0
        } else {
            self.base.scale()
        };
        self.scale.unwrap_or(1.0) * extra_scale
    }

    pub fn map_widget_pos_to_content_pos(&self, pos: Point) -> Point {
        let scale = self.total_scale();
        Point::new(pos.x().div_f32_round(scale), pos.y().div_f32_round(scale))
    }
}

impl NewWidget for Image {
    type Arg = Option<Pixmap>;

    fn new(base: WidgetBaseOf<Self>, arg: Self::Arg) -> Self {
        Self {
            base,
            pixmap: arg,
            is_prescaled: false,
            scale: None,
        }
    }

    fn handle_declared(&mut self, arg: Self::Arg) {
        self.set_pixmap(arg);
    }
}

impl Widget for Image {
    impl_widget_base!();

    fn handle_draw(&mut self, event: DrawEvent) -> Result<()> {
        let scale = self.total_scale();
        if let Some(pixmap) = &self.pixmap {
            event.draw_pixmap(
                Point::default(),
                pixmap.as_tiny_skia_ref(),
                Transform::from_scale(scale, scale),
            );
        }
        Ok(())
    }

    fn handle_size_hint_x_request(&self) -> Result<SizeHint> {
        let scale = self.total_scale();
        let size = self
            .pixmap
            .as_ref()
            .map_or(0.ppx(), |p| p.size_x().mul_f32_ceil(scale));

        Ok(SizeHint::new_fixed(size, size))
    }

    fn handle_size_hint_y_request(&self, _size_x: PhysicalPixels) -> Result<SizeHint> {
        let scale = self.total_scale();
        let size = self
            .pixmap
            .as_ref()
            .map_or(0.ppx(), |p| p.size_y().mul_f32_ceil(scale));

        Ok(SizeHint::new_fixed(size, size))
    }

    fn handle_layout(&mut self, _event: crate::event::LayoutEvent) -> Result<()> {
        Ok(())
    }
}
