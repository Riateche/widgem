use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        ChildKey, WidgetBase,
    },
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    pub fn new(base: WidgetBaseOf<Self>) -> Self {
        Self { base }
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

impl Widget for RootWidget {
    impl_widget_base!();
}
