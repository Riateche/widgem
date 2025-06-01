use {
    super::Widget,
    crate::callback::{widget_callback, Callback},
    accesskit::NodeId,
    anyhow::Result,
    std::{
        fmt::{self, Debug},
        marker::PhantomData,
        sync::atomic::{AtomicU64, Ordering},
    },
};

/// Raw (untyped) widget ID.
///
/// This ID may refer to a widget of any type.
///
/// Existence of an ID does not guarantee existence of the corresponding widget.
/// Widgets can be deleted at any time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawWidgetId(u64);

impl RawWidgetId {
    /// Allocates a new widget ID.
    ///
    /// You shouldn't need to use this function.
    pub fn new_unique() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn callback<W, E, F>(self, func: F) -> Callback<E>
    where
        W: Widget,
        F: Fn(&mut W, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(WidgetId::<W>::new(self), func)
    }
}

impl From<RawWidgetId> for NodeId {
    fn from(value: RawWidgetId) -> Self {
        value.0.into()
    }
}

impl From<NodeId> for RawWidgetId {
    fn from(value: NodeId) -> Self {
        RawWidgetId(value.into())
    }
}

pub struct WidgetId<T>(RawWidgetId, PhantomData<T>);

impl<T> WidgetId<T> {
    pub fn new(id: RawWidgetId) -> Self {
        Self(id, PhantomData)
    }

    pub fn raw(self) -> RawWidgetId {
        self.0
    }

    pub fn callback<E, F>(self, func: F) -> Callback<E>
    where
        T: Widget,
        F: Fn(&mut T, E) -> Result<()> + 'static,
        E: 'static,
    {
        widget_callback(self, func)
    }
}

impl<T> Debug for WidgetId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl<T> Clone for WidgetId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for WidgetId<T> {}

pub struct WidgetWithId<W> {
    pub id: WidgetId<W>,
    pub widget: W,
}

impl<T> From<WidgetId<T>> for RawWidgetId {
    fn from(value: WidgetId<T>) -> Self {
        value.raw()
    }
}

impl<T> From<WidgetId<T>> for NodeId {
    fn from(value: WidgetId<T>) -> Self {
        value.raw().into()
    }
}
