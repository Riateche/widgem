use {
    super::{Widget, WidgetBaseOf},
    crate::{
        impl_widget_base,
        widgets::widget_trait::{WidgetInitializer},
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
