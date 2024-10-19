use anyhow::{Context, Result};
use salvation_macros::impl_with;

use crate::{
    impl_widget_common,
    layout::{Alignment, LayoutItemOptions},
};

use super::{Widget, WidgetCommon};

pub struct Row {
    // TODO: add layout options
    common: WidgetCommon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Options {
    pub alignment: Option<Alignment>,
    // TODO: alignment, priority, stretch, etc.
}

#[impl_with]
impl Row {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            common: WidgetCommon::new::<Self>().into(),
        }
    }

    pub fn add_child(&mut self, widget: Box<dyn Widget>) {
        let row = self.common.children.len();
        self.common
            .add_child(widget, LayoutItemOptions::from_pos_in_grid(row as i32, 0));
        self.common.update();
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

impl Widget for Row {
    impl_widget_common!();
}
