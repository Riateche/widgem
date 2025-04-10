use {
    super::{Widget, WidgetCommon, WidgetCommonTyped},
    crate::{
        impl_widget_common,
        layout::{Alignment, LayoutItemOptions},
    },
    anyhow::{Context, Result},
    salvation_macros::impl_with,
};

pub struct Column {
    // TODO: add layout options
    common: WidgetCommon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Options {
    pub alignment: Option<Alignment>,
    // TODO: alignment, priority, stretch, etc.
}

#[impl_with]
impl Column {
    pub fn add_child<T: Widget>(&mut self) -> &mut T {
        let row = self.common.children.len();
        self.common
            .add_child::<T>(LayoutItemOptions::from_pos_in_grid(0, row as i32))
    }

    pub fn set_options(&mut self, index: usize, options: Options) -> Result<()> {
        let mut all_options = self
            .common
            .children
            .get(index)
            .context("invalid child index")?
            .options
            .clone();
        all_options.x.alignment = options.alignment;
        self.common.set_child_options(index, all_options)
    }

    pub fn and_options(mut self, options: Options) -> Self {
        let index = self.common.children.len();
        self.set_options(index, options)
            .expect("should not fail with correct index");
        self
    }
}

impl Widget for Column {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
        }
    }
}
