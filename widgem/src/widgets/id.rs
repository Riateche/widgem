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

    // TODO: add example

    /// Creates a callback that will call `func` on the receiver widget with ID `self`.
    ///
    /// The callback will only be invoked after you register it by passing it to a
    /// `.on_*()` function of the sender widget.
    ///
    /// It's only possible to register a single callback for a given sender-signal-receiver
    /// triplet. If another callback with the same receiver is supplied to the same
    /// `.on_*()` function of the same sender widget, it will replace the previous callback.
    /// Thus, it is save to call `.on_*()` functions within `handle_declare_children_request`,
    /// as the new callbacks will overwrite the old ones instead of creating new copies.
    ///
    /// The callback will be automatically deregistered when the sender or the receiver is deleted.
    pub fn callback<E, F>(self, func: F) -> Callback<E>
    where
        T: Widget,
        F: Fn(&mut T, E) -> Result<()> + 'static,
        E: 'static,
    {
        // TODO: add a way to add raw widget callbacks (i.e. operating on &mut dyn Widget)
        widget_callback(self, func)
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
