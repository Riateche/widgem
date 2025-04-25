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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawWidgetId(pub u64);

impl RawWidgetId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
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

pub struct WidgetId<T>(pub RawWidgetId, pub PhantomData<T>);

impl<T> WidgetId<T> {
    pub fn new(id: RawWidgetId) -> Self {
        Self(id, PhantomData)
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
