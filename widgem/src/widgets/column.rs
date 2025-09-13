use {
    super::{Widget, WidgetBaseOf},
    crate::{impl_widget_base, widgets::widget_trait::NewWidget},
};

pub struct Column {
    // TODO: add layout options
    base: WidgetBaseOf<Self>,
}

impl NewWidget for Column {
    type Arg = ();

    fn new(base: WidgetBaseOf<Self>, (): Self::Arg) -> Self {
        Self { base }
    }
    fn handle_declared(&mut self, (): Self::Arg) {}
}

impl Widget for Column {
    impl_widget_base!();
}

/*
pub struct ColumnChildrenWithKeyHandle<'a, ChildKeyType: Into<ChildKey>> {
    base: &'a mut WidgetBase,
    already_set: HashSet<ChildKey>,
    _marker: PhantomData<fn() -> ChildKeyType>,
}

impl<'a, ChildKeyType: Into<ChildKey>> ColumnChildrenWithKeyHandle<'a, ChildKeyType> {
    pub fn add_child<T: NewWidget>(&mut self, _key: ChildKeyType, _arg: T::Arg) -> &mut T {
        todo!()
    }

    pub fn has_child(&mut self, _key: &ChildKeyType) -> bool {
        todo!()
    }

    pub fn remove_child(&mut self, _key: &ChildKeyType) {
        todo!()
    }

    pub fn remove_others(&mut self) {}
}

pub struct ColumnChildrenHandle<'a> {
    base: &'a mut WidgetBase,
    index: usize,
}

impl<'a> ColumnChildrenHandle<'a> {
    pub fn add_child<T: NewWidget>(&mut self, _arg: T::Arg) -> &mut T {
        todo!()
    }

    pub fn remove_others(&mut self) {}
}
*/
