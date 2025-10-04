use {
    crate::{widget_initializer::WidgetInitializer, ChildKey, Widget, WidgetBase, WidgetNotFound},
    std::{borrow::Borrow, collections::HashSet, ops::Deref},
};

pub struct Items<BaseRef: Borrow<WidgetBase>> {
    pub base: BaseRef,
}

impl<BaseRef: Borrow<WidgetBase>> Items<BaseRef> {
    pub fn new(base: BaseRef) -> Self {
        Self { base }
    }

    pub fn has_item(&mut self, index: u32) -> bool {
        self.base.borrow().has_child(index)
    }

    pub fn item<T: Widget>(&self, index: u32) -> anyhow::Result<&T> {
        self.base.borrow().get_child(index)
    }

    pub fn dyn_item(&self, index: u32) -> anyhow::Result<&dyn Widget> {
        self.base.borrow().get_dyn_child(index)
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

pub struct ItemsMut<'a> {
    inner: Items<&'a mut WidgetBase>,
    next_index: u32,
    already_set: HashSet<ChildKey>,
}

impl<'a> Deref for ItemsMut<'a> {
    type Target = Items<&'a mut WidgetBase>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> ItemsMut<'a> {
    pub fn new(base: &'a mut WidgetBase) -> Self {
        Self {
            inner: Items::new(base),
            next_index: 0,
            already_set: HashSet::new(),
        }
    }

    pub fn set_next_item<WI: WidgetInitializer>(&mut self, initializer: WI) -> &mut WI::Output {
        let key = ChildKey::from(self.next_index);
        let output = self.inner.base.set_child(key.clone(), initializer);
        self.already_set.insert(key);
        self.next_index += 1;
        output
    }

    pub fn set_item_at<WI: WidgetInitializer>(
        &mut self,
        index: u32,
        initializer: WI,
    ) -> &mut WI::Output {
        let key = ChildKey::from(index);
        let output = self.inner.base.set_child(key.clone(), initializer);
        self.already_set.insert(key);
        self.next_index = index + 1;
        output
    }

    pub fn remove_item(&mut self, index: u32) -> Result<(), WidgetNotFound> {
        self.inner.base.remove_child(index)
    }

    pub fn remove_other_items(&mut self) {
        self.inner.base.remove_children_except(&self.already_set);
    }

    pub fn item_mut<T: Widget>(&mut self, index: u32) -> anyhow::Result<&mut T> {
        self.inner.base.get_child_mut(index)
    }

    pub fn dyn_item_mut(&mut self, index: u32) -> anyhow::Result<&mut dyn Widget> {
        self.inner.base.get_dyn_child_mut(index)
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

    pub fn next_index(&self) -> u32 {
        self.next_index
    }
}
