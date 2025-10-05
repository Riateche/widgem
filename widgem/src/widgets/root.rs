use crate::{
    impl_widget_base,
    items::{
        with_index::{Items, ItemsMut},
        with_key::{ItemsWithKey, ItemsWithKeyMut},
    },
    widget_initializer::WidgetInitializer,
    ChildKey, Widget, WidgetBase, WidgetBaseOf,
};

pub struct RootWidget {
    base: WidgetBaseOf<Self>,
}

impl RootWidget {
    pub fn new(base: WidgetBaseOf<Self>) -> Self {
        Self { base }
    }

    pub fn set_main_content<WI: WidgetInitializer>(
        &mut self,
        initializer: WI,
    ) -> anyhow::Result<&mut WI::Output> {
        self.base.set_main_child(initializer)
    }

    pub fn contents(&self) -> Items<&WidgetBase> {
        Items::new(&self.base)
    }

    pub fn content_mut(&mut self) -> ItemsMut<'_> {
        ItemsMut::new(&mut self.base)
    }

    pub fn contents_with_key<K: Into<ChildKey>>(&self) -> ItemsWithKey<&WidgetBase, K> {
        ItemsWithKey::new(&self.base)
    }

    pub fn contents_with_key_mut<K: Into<ChildKey>>(&mut self) -> ItemsWithKeyMut<'_, K> {
        ItemsWithKeyMut::new(&mut self.base)
    }
}

impl Widget for RootWidget {
    impl_widget_base!();
}
