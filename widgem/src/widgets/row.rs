use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        layout::Layout,
        widget_initializer::{self, WidgetInitializer},
        ChildKey, WidgetBase,
    },
};

// TODO: reimplement auto keys and auto row/column
pub struct Row {
    base: WidgetBaseOf<Self>,
}

impl Row {
    fn new(mut base: WidgetBaseOf<Self>) -> Self {
        base.set_layout(Layout::HorizontalFirst);
        Self { base }
    }

    pub fn init() -> impl WidgetInitializer<Output = Self> {
        widget_initializer::from_new(Self::new)
    }

    pub fn contents(&self) -> Items<&WidgetBase> {
        Items::new(&self.base)
    }

    pub fn contents_mut(&mut self) -> ItemsMut<'_> {
        ItemsMut::new(&mut self.base)
    }

    pub fn contents_with_key<K: Into<ChildKey>>(&self) -> ItemsWithKey<&WidgetBase, K> {
        ItemsWithKey::new(&self.base)
    }

    pub fn contents_with_key_mut<K: Into<ChildKey>>(&mut self) -> ItemsWithKeyMut<'_, K> {
        ItemsWithKeyMut::new(&mut self.base)
    }
}

impl Widget for Row {
    impl_widget_base!();
}
