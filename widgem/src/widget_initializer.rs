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
    fn init(self, base: WidgetBaseOf<Self::Output>) -> anyhow::Result<Self::Output>;

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
    fn reinit(self, widget: &mut Self::Output) -> anyhow::Result<()>;
}

pub fn from_new<W, F>(new_fn: F) -> impl WidgetInitializer<Output = W>
where
    F: Fn(WidgetBaseOf<W>) -> W,
    W: Widget,
{
    struct Initializer<W, F> {
        new_fn: F,
        _marker: PhantomData<fn() -> W>,
    }

    impl<W, F> WidgetInitializer for Initializer<W, F>
    where
        F: Fn(WidgetBaseOf<W>) -> W,
        W: Widget,
    {
        type Output = W;

        fn init(self, base: WidgetBaseOf<Self::Output>) -> anyhow::Result<Self::Output> {
            Ok((self.new_fn)(base))
        }

        fn reinit(self, _widget: &mut Self::Output) -> anyhow::Result<()> {
            Ok(())
        }
    }

    Initializer {
        new_fn,
        _marker: PhantomData,
    }
}

pub fn from_fallible_new<W, F>(new_fn: F) -> impl WidgetInitializer<Output = W>
where
    F: Fn(WidgetBaseOf<W>) -> anyhow::Result<W>,
    W: Widget,
{
    struct Initializer<W, F> {
        new_fn: F,
        _marker: PhantomData<fn() -> W>,
    }

    impl<W, F> WidgetInitializer for Initializer<W, F>
    where
        F: Fn(WidgetBaseOf<W>) -> anyhow::Result<W>,
        W: Widget,
    {
        type Output = W;

        fn init(self, base: WidgetBaseOf<Self::Output>) -> anyhow::Result<Self::Output> {
            (self.new_fn)(base)
        }

        fn reinit(self, _widget: &mut Self::Output) -> anyhow::Result<()> {
            Ok(())
        }
    }

    Initializer {
        new_fn,
        _marker: PhantomData,
    }
}

pub fn from_new_and_set<W, A, NF, SF>(
    new_fn: NF,
    set_fn: SF,
    arg: A,
) -> impl WidgetInitializer<Output = W>
where
    NF: Fn(WidgetBaseOf<W>, A) -> W,
    SF: Fn(&mut W, A) -> &mut W,
    W: Widget,
{
    struct Initializer<W, A, NF, SF> {
        arg: A,
        new_fn: NF,
        set_fn: SF,
        _marker: PhantomData<fn() -> W>,
    }

    impl<W, A, NF, SF> WidgetInitializer for Initializer<W, A, NF, SF>
    where
        NF: Fn(WidgetBaseOf<W>, A) -> W,
        SF: Fn(&mut W, A) -> &mut W,
        W: Widget,
    {
        type Output = W;

        fn init(self, base: WidgetBaseOf<Self::Output>) -> anyhow::Result<Self::Output> {
            Ok((self.new_fn)(base, self.arg))
        }

        fn reinit(self, widget: &mut Self::Output) -> anyhow::Result<()> {
            (self.set_fn)(widget, self.arg);
            Ok(())
        }
    }

    Initializer {
        new_fn,
        set_fn,
        arg,
        _marker: PhantomData,
    }
}

pub fn from_fallible_new_and_set<W, A, NF, SF>(
    new_fn: NF,
    set_fn: SF,
    arg: A,
) -> impl WidgetInitializer<Output = W>
where
    NF: Fn(WidgetBaseOf<W>, A) -> anyhow::Result<W>,
    SF: Fn(&mut W, A) -> &mut W,
    W: Widget,
{
    struct Initializer<W, A, NF, SF> {
        arg: A,
        new_fn: NF,
        set_fn: SF,
        _marker: PhantomData<fn() -> W>,
    }

    impl<W, A, NF, SF> WidgetInitializer for Initializer<W, A, NF, SF>
    where
        NF: Fn(WidgetBaseOf<W>, A) -> anyhow::Result<W>,
        SF: Fn(&mut W, A) -> &mut W,
        W: Widget,
    {
        type Output = W;

        fn init(self, base: WidgetBaseOf<Self::Output>) -> anyhow::Result<Self::Output> {
            (self.new_fn)(base, self.arg)
        }

        fn reinit(self, widget: &mut Self::Output) -> anyhow::Result<()> {
            (self.set_fn)(widget, self.arg);
            Ok(())
        }
    }

    Initializer {
        new_fn,
        set_fn,
        arg,
        _marker: PhantomData,
    }
}

pub fn from_fallible_new_and_fallible_set<W, A, NF, SF>(
    new_fn: NF,
    set_fn: SF,
    arg: A,
) -> impl WidgetInitializer<Output = W>
where
    NF: Fn(WidgetBaseOf<W>, A) -> anyhow::Result<W>,
    SF: Fn(&mut W, A) -> anyhow::Result<&mut W>,
    W: Widget,
{
    struct Initializer<W, A, NF, SF> {
        arg: A,
        new_fn: NF,
        set_fn: SF,
        _marker: PhantomData<fn() -> W>,
    }

    impl<W, A, NF, SF> WidgetInitializer for Initializer<W, A, NF, SF>
    where
        NF: Fn(WidgetBaseOf<W>, A) -> anyhow::Result<W>,
        SF: Fn(&mut W, A) -> anyhow::Result<&mut W>,
        W: Widget,
    {
        type Output = W;

        fn init(self, base: WidgetBaseOf<Self::Output>) -> anyhow::Result<Self::Output> {
            (self.new_fn)(base, self.arg)
        }

        fn reinit(self, widget: &mut Self::Output) -> anyhow::Result<()> {
            (self.set_fn)(widget, self.arg)?;
            Ok(())
        }
    }

    Initializer {
        new_fn,
        set_fn,
        arg,
        _marker: PhantomData,
    }
}

pub fn from_fallible_new_and_fallible_2_set<W, A1, A2, NF, SF1, SF2>(
    new_fn: NF,
    set_fn1: SF1,
    set_fn2: SF2,
    arg1: A1,
    arg2: A2,
) -> impl WidgetInitializer<Output = W>
where
    NF: Fn(WidgetBaseOf<W>, A1, A2) -> anyhow::Result<W>,
    SF1: Fn(&mut W, A1) -> anyhow::Result<&mut W>,
    SF2: Fn(&mut W, A2) -> anyhow::Result<&mut W>,
    W: Widget,
{
    struct Initializer<W, A1, A2, NF, SF1, SF2> {
        arg1: A1,
        arg2: A2,
        new_fn: NF,
        set_fn1: SF1,
        set_fn2: SF2,
        _marker: PhantomData<fn() -> W>,
    }

    impl<W, A1, A2, NF, SF1, SF2> WidgetInitializer for Initializer<W, A1, A2, NF, SF1, SF2>
    where
        NF: Fn(WidgetBaseOf<W>, A1, A2) -> anyhow::Result<W>,
        SF1: Fn(&mut W, A1) -> anyhow::Result<&mut W>,
        SF2: Fn(&mut W, A2) -> anyhow::Result<&mut W>,
        W: Widget,
    {
        type Output = W;

        fn init(self, base: WidgetBaseOf<Self::Output>) -> anyhow::Result<Self::Output> {
            (self.new_fn)(base, self.arg1, self.arg2)
        }

        fn reinit(self, widget: &mut Self::Output) -> anyhow::Result<()> {
            (self.set_fn1)(widget, self.arg1)?;
            (self.set_fn2)(widget, self.arg2)?;
            Ok(())
        }
    }

    Initializer {
        new_fn,
        set_fn1,
        set_fn2,
        arg1,
        arg2,
        _marker: PhantomData,
    }
}
