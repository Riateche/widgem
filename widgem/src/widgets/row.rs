use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        items::{
            with_index::{Items, ItemsMut},
            with_key::{ItemsWithKey, ItemsWithKeyMut},
        },
        layout::Layout,
        widget_initializer::WidgetInitializer,
        ChildKey, WidgetBase,
    },
};

// TODO: reimplement auto keys and auto row/column
pub struct Row {
    base: WidgetBaseOf<Self>,
}

impl Row {
    pub fn init() -> impl WidgetInitializer<Output = Self> {
        Initializer
    }

    // pub fn set_main_content<WI: WidgetInitializer>(&mut self, initializer: WI) -> &mut WI::Output {
    //     self.base.set_main_child(initializer)
    // }

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

struct Initializer;

impl WidgetInitializer for Initializer {
    type Output = Row;

    fn init(self, mut base: WidgetBaseOf<Self::Output>) -> Self::Output {
        base.set_layout(Layout::HorizontalFirst);
        Row { base }
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

impl Widget for Row {
    impl_widget_base!();
}
