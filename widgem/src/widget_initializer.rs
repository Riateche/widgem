use {super::WidgetBaseOf, crate::Widget, std::marker::PhantomData};

pub trait WidgetInitializer {
    type Output: Widget;
    /// Creates a new widget. The `base` argument provides all available information about the context in which
    /// the widget is being created.
    ///
    /// You don't need to call this function directly. It's automatically invoked when you create a widget using
    /// one of the following functions on [WidgetBase] of the parent widget:
    /// - [WidgetBase::set_child]
    /// - [crate::items::with_index::ItemsMut::set_item_at]
    /// - [crate::items::with_index::ItemsMut::set_next_item]
    /// - [crate::items::with_key::ItemsWithKeyMut::set_item]
    ///
    /// When implementing this function, you should always store the `common` argument value inside your widget object.
    /// As a convention, you should store it in the widget's field named `common`.
    /// Your implementations of [base](Widget::base) and [base_mut](Widget::base_mut) must return a reference to that object.
    fn init(self, base: WidgetBaseOf<Self::Output>) -> Self::Output;

    /// Handles a repeated declaration of the widget.
    ///
    /// This function may be called from the following functions when the corresponding widget already exist:
    ///
    /// - [WidgetBase::set_child]
    /// - [crate::items::with_index::ItemsMut::set_item_at]
    /// - [crate::items::with_index::ItemsMut::set_next_item]
    /// - [crate::items::with_key::ItemsWithKeyMut::set_item]
    ///
    /// When implementing this function, update the configuration of the widget based on the data stored in the initializer.
    /// For example, `Label`'s initializer contains the text property, so its implementation of `reinit` calls `Label::set_text`.
    fn reinit(self, widget: &mut Self::Output);
}

pub struct WidgetInitializerNoArg<W, F>
where
    F: Fn(WidgetBaseOf<W>) -> W,
{
    new_fn: F,
    _marker: PhantomData<fn() -> W>,
}

impl<W, F> WidgetInitializerNoArg<W, F>
where
    F: Fn(WidgetBaseOf<W>) -> W,
{
    pub fn new(new_fn: F) -> Self {
        Self {
            new_fn,
            _marker: PhantomData,
        }
    }
}

impl<W, F> WidgetInitializer for WidgetInitializerNoArg<W, F>
where
    F: Fn(WidgetBaseOf<W>) -> W,
    W: Widget,
{
    type Output = W;

    fn init(self, base: WidgetBaseOf<Self::Output>) -> Self::Output {
        (self.new_fn)(base)
    }

    fn reinit(self, _widget: &mut Self::Output) {}
}

pub struct WidgetInitializerOneArg<W, A, NF, SF>
where
    NF: Fn(WidgetBaseOf<W>, A) -> W,
    SF: Fn(&mut W, A),
{
    arg: A,
    new_fn: NF,
    set_fn: SF,
    _marker: PhantomData<fn() -> W>,
}

impl<W, A, NF, SF> WidgetInitializerOneArg<W, A, NF, SF>
where
    NF: Fn(WidgetBaseOf<W>, A) -> W,
    SF: Fn(&mut W, A),
{
    pub fn new(new_fn: NF, set_fn: SF, arg: A) -> Self {
        Self {
            new_fn,
            set_fn,
            arg,
            _marker: PhantomData,
        }
    }
}

impl<W, A, NF, SF> WidgetInitializer for WidgetInitializerOneArg<W, A, NF, SF>
where
    NF: Fn(WidgetBaseOf<W>, A) -> W,
    SF: Fn(&mut W, A),
    W: Widget,
{
    type Output = W;

    fn init(self, base: WidgetBaseOf<Self::Output>) -> Self::Output {
        (self.new_fn)(base, self.arg)
    }

    fn reinit(self, widget: &mut Self::Output) {
        (self.set_fn)(widget, self.arg)
    }
}
