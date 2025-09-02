use {
    accesskit::NodeId,
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
/// Existence of an ID does not guarantee that the corresponding widget exists.
/// Widgets can be deleted at any time.
/// Missing widget errors should be handled gracefully.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawWidgetId(u64);

impl RawWidgetId {
    /// Allocates a new widget ID.
    ///
    /// You shouldn't need to use this function directly.
    pub fn new_unique() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
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

/// Widget ID that references a widget of type `T`.
///
/// Existence of an ID does not guarantee that the corresponding widget exists or that
/// it has the indicated type.
/// Widgets can be deleted at any time.
/// Missing widget errors should be handled gracefully.
pub struct WidgetId<T>(RawWidgetId, PhantomData<fn() -> T>);

impl<T> WidgetId<T> {
    /// Creates a new typed widget ID from an untyped ID.
    ///
    /// You shouldn't need to use this function directly.
    pub fn new(id: RawWidgetId) -> Self {
        Self(id, PhantomData)
    }

    /// Converts a typed widget ID into an untyped ID. You can also use `.into()`.
    pub fn raw(self) -> RawWidgetId {
        self.0
    }
}

impl<T> Debug for WidgetId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WidgetId<{}>({:?})",
            std::any::type_name::<T>(),
            self.0 .0,
        )
    }
}

impl<T> Clone for WidgetId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for WidgetId<T> {}

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
