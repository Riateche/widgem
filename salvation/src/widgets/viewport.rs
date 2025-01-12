use {
    super::{Widget, WidgetCommon},
    crate::{impl_widget_common, layout::SizeHintMode},
    anyhow::Result,
};

// TODO: public type for empty widget?
pub struct Viewport {
    common: WidgetCommon,
}

impl Viewport {
    pub fn new() -> Self {
        Self {
            common: WidgetCommon::new::<Self>().into(),
        }
    }
}

impl Widget for Viewport {
    impl_widget_common!();

    fn recalculate_size_hint_x(&mut self, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }
    fn recalculate_size_hint_y(&mut self, _size_x: i32, _mode: SizeHintMode) -> Result<i32> {
        Ok(0)
    }
    fn recalculate_size_x_fixed(&mut self) -> bool {
        false
    }
    fn recalculate_size_y_fixed(&mut self) -> bool {
        false
    }
}
