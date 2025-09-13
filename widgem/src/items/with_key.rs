use {
    crate::{widgets::WidgetInitializer, ChildKey, Widget, WidgetBase, WidgetNotFound},
    std::{borrow::Borrow, collections::HashSet, marker::PhantomData, ops::Deref},
};

pub struct ItemsWithKey<BaseRef: Borrow<WidgetBase>, ChildKeyType: Into<ChildKey>> {
    base: BaseRef,
    _marker: PhantomData<fn() -> ChildKeyType>,
}

impl<BaseRef: Borrow<WidgetBase>, ChildKeyType: Into<ChildKey>>
    ItemsWithKey<BaseRef, ChildKeyType>
{
    pub fn new(base: BaseRef) -> Self {
        ItemsWithKey {
            base,
            _marker: PhantomData,
        }
    }

    pub fn has_item(&mut self, key: ChildKeyType) -> bool {
        self.base.borrow().has_child(key)
    }

    pub fn item<T: Widget>(&self, key: ChildKeyType) -> anyhow::Result<&T> {
        self.base.borrow().get_child(key)
    }

    pub fn dyn_item(&self, key: ChildKeyType) -> anyhow::Result<&dyn Widget> {
        self.base.borrow().get_dyn_child(key)
    }

    pub fn all_items(&self) -> impl Iterator<Item = &dyn Widget> {
        self.base.borrow().children()
    }

    /// Returns an iterator over the widget's children and associated keys.
    pub fn all_items_with_keys(&self) -> impl Iterator<Item = (&ChildKey, &dyn Widget)> {
        self.base.borrow().children_with_keys()
    }

    /// Returns an iterator over the keys of the widget's children.
    pub fn all_item_keys(&self) -> impl Iterator<Item = &ChildKey> {
        self.base.borrow().child_keys()
    }
}

pub struct ItemsWithKeyMut<'a, ChildKeyType: Into<ChildKey>> {
    inner: ItemsWithKey<&'a mut WidgetBase, ChildKeyType>,
    already_set: HashSet<ChildKey>,
}

impl<'a, ChildKeyType: Into<ChildKey>> Deref for ItemsWithKeyMut<'a, ChildKeyType> {
    type Target = ItemsWithKey<&'a mut WidgetBase, ChildKeyType>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, ChildKeyType: Into<ChildKey>> ItemsWithKeyMut<'a, ChildKeyType> {
    pub fn new(base: &'a mut WidgetBase) -> Self {
        Self {
            inner: ItemsWithKey::new(base),
            already_set: HashSet::new(),
        }
    }

    pub fn set_item<WI: WidgetInitializer>(
        &mut self,
        key: ChildKeyType,
        initializer: WI,
    ) -> &mut WI::Output {
        self.inner.base.set_child(key, initializer)
    }

    pub fn remove_item(&mut self, key: ChildKeyType) -> Result<(), WidgetNotFound> {
        self.inner.base.remove_child(key)
    }

    pub fn remove_other_items(&mut self) {
        self.inner.base.remove_children_except(&self.already_set);
    }

    pub fn item_mut<T: Widget>(&mut self, key: ChildKeyType) -> anyhow::Result<&mut T> {
        self.inner.base.get_child_mut(key)
    }

    pub fn dyn_item_mut(&mut self, key: ChildKeyType) -> anyhow::Result<&mut dyn Widget> {
        self.inner.base.get_dyn_child_mut(key)
    }

    /// Returns an iterator over the widget's children.
    pub fn all_items_mut(&mut self) -> impl Iterator<Item = &mut dyn Widget> {
        self.inner.base.children_mut()
    }

    /// Returns an iterator over the widget's children and associated keys.
    pub fn all_items_with_keys_mut(
        &mut self,
    ) -> impl Iterator<Item = (&ChildKey, &mut dyn Widget)> {
        self.inner.base.children_with_keys_mut()
    }
}
