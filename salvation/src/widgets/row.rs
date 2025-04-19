use {
    super::{Key, Widget, WidgetCommon, WidgetCommonTyped},
    crate::{
        impl_widget_common,
        layout::{Alignment, LayoutItemOptions},
    },
    anyhow::{Context, Result},
    salvation_macros::impl_with,
};

pub struct Row {
    // TODO: add layout options
    common: WidgetCommon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Options {
    pub alignment: Option<Alignment>,
    pub is_fixed: Option<bool>,
    // TODO: alignment, priority, stretch, etc.
}

#[impl_with]
impl Row {
    pub fn add_child<T: Widget>(&mut self) -> &mut T {
        let row = self.common.children.len();
        self.common.add_child::<T>(
            row as u64,
            LayoutItemOptions::from_pos_in_grid(row as i32, 0),
        )
    }

    pub fn set_options(&mut self, key: Key, options: Options) -> Result<()> {
        let mut all_options = self
            .common
            .children
            .get(&key)
            .context("invalid child index")?
            .options
            .clone();
        all_options.x.alignment = options.alignment;
        all_options.x.is_fixed = options.is_fixed;
        self.common.set_child_options(key, all_options)
    }

    // pub fn and_options(mut self, options: Options) -> Self {
    //     let index = self.common.children.len();
    //     self.set_options(index, options)
    //         .expect("should not fail with correct index");
    //     self
    // }
}

impl Widget for Row {
    impl_widget_common!();

    fn new(common: WidgetCommonTyped<Self>) -> Self {
        Self {
            common: common.into(),
        }
    }
}
