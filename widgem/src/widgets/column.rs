use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        widgets::widget_trait::WidgetInitializer,
        ChildKey, WidgetBase,
    },
};

pub struct Column {
    // TODO: add layout options
    base: WidgetBaseOf<Self>,
}

impl Column {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        Initializer
    }

    pub fn items(&self) -> Items<&WidgetBase> {
        Items::new(&self.base)
    }

    pub fn items_mut(&mut self) -> ItemsMut<'_> {
        ItemsMut::new(&mut self.base)
    }

    pub fn items_with_key<K: Into<ChildKey>>(&self) -> ItemsWithKey<&WidgetBase, K> {
        ItemsWithKey::new(&self.base)
    }

    pub fn items_with_key_mut<K: Into<ChildKey>>(&mut self) -> ItemsWithKeyMut<'_, K> {
        ItemsWithKeyMut::new(&mut self.base)
    }
}

struct Initializer;

impl WidgetInitializer for Initializer {
    type Output = Column;

    fn init(self, base: WidgetBaseOf<Self::Output>) -> Self::Output {
        Column { base }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for Column {
    impl_widget_base!();
}
